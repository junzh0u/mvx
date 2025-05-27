use anyhow::bail;
use anyhow::ensure;
use clap::Parser;
use fs_extra::file::{move_file_with_progress, CopyOptions, TransitProcess};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Suppress progress bars and messages
    #[arg(short, long)]
    quiet: bool,

    src: PathBuf,
    dest: PathBuf,
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let start = Instant::now();
    let mp = MultiProgress::new();
    if cli.quiet {
        mp.set_draw_target(ProgressDrawTarget::hidden());
    }
    let pb_info = mp.add(
        ProgressBar::new_spinner()
            .with_style(ProgressStyle::default_spinner().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")),
    );
    pb_info.enable_steady_tick(std::time::Duration::from_millis(100));

    let src = &cli.src;
    let mut dest = cli.dest.clone();
    ensure!(
        src.exists(),
        "Source path '{}' does not exist.",
        src.display()
    );
    if src.is_file() {
        if dest.is_dir() || (!dest.exists() && dest.to_string_lossy().ends_with('/')) {
            dest.push(src.file_name().unwrap());
        }
        pb_info.set_message(format!(
            "Moving '{}' to '{}'",
            src.display(),
            dest.display()
        ));
        move_file(src, &dest, &mp)?;
        pb_info.set_style(ProgressStyle::default_spinner().template("{prefix:.bold.green} {msg}")?);
        pb_info.set_prefix("✔");
        pb_info.finish_with_message(format!(
            "Moved '{}' to '{}' in {}",
            src.display(),
            dest.display(),
            HumanDuration(start.elapsed())
        ));
    } else if src.is_dir() {
        if dest.exists() {
            ensure!(
                dest.is_dir(),
                "Destination path is not a directory: '{}'",
                dest.display()
            );
        } else {
            fs::create_dir_all(&dest)?;
        }
        pb_info.set_message(format!(
            "Merging '{}' into '{}'",
            src.display(),
            dest.display()
        ));
        merge_directories(src, &dest, &mp)?;
        pb_info.set_style(ProgressStyle::default_spinner().template("{prefix:.bold.green} {msg}")?);
        pb_info.set_prefix("✔");
        pb_info.finish_with_message(format!(
            "Merged '{}' into '{}' in {}",
            src.display(),
            dest.display(),
            HumanDuration(start.elapsed())
        ));
    } else {
        bail!(
            "Source path is neither a file nor directory: '{}'",
            src.display()
        )
    }
    Ok(())
}

fn merge_directories(src: &PathBuf, dest: &Path, mp: &MultiProgress) -> anyhow::Result<()> {
    let files = collect_files(src)?;
    let pb_files = mp.add(
        ProgressBar::new(files.len() as u64).with_style(
            ProgressStyle::with_template("[{bar:40.cyan/blue}] {pos}/{len} {msg}")?
                .progress_chars("=>-"),
        ),
    );

    for file in files {
        let rel_path = file.strip_prefix(src)?;
        let dest_file = dest.join(rel_path);
        pb_files.set_message(rel_path.display().to_string());

        move_file(&file, &dest_file, mp)?;

        pb_files.inc(1);
    }
    fs::remove_dir_all(src)?;

    pb_files.finish_and_clear();
    mp.remove(&pb_files);

    Ok(())
}

fn move_file(src: &PathBuf, dest: &PathBuf, mp: &MultiProgress) -> anyhow::Result<()> {
    let parent = dest.parent().unwrap();
    fs::create_dir_all(parent)?;

    let src_meta = fs::metadata(src)?;
    let dest_meta = fs::metadata(parent)?;
    if src_meta.dev() == dest_meta.dev() {
        fs::rename(src, dest)?;
    } else {
        let pb_bytes = mp.add(
            ProgressBar::new(src_meta.len()).with_style(
                ProgressStyle::default_bar()
                    .template("[{bar:40.green/white}] {bytes}/{total_bytes} [{bytes_per_sec}] (ETA: {eta})")?
                    .progress_chars("=>-"),
            ),
        );
        let progress_handler = |transit: TransitProcess| {
            pb_bytes.set_position(transit.copied_bytes);
        };
        let options = CopyOptions::new().overwrite(true);
        move_file_with_progress(src, dest, &options, progress_handler)?;
        pb_bytes.finish_and_clear();
        mp.remove(&pb_bytes);
    }
    Ok(())
}

fn collect_files(dir: &PathBuf) -> std::io::Result<Vec<std::path::PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|path| {
            if path.is_dir() {
                collect_files(&path).unwrap()
            } else if path.is_file() {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect())
}
