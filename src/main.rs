use fs_extra::file::{move_file_with_progress, CopyOptions, TransitProcess};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Suppress progress bar and other output
    #[arg(short, long)]
    quiet: bool,

    src: PathBuf,
    dest: PathBuf,
}

fn main() {
    let args = Args::parse();

    merge(&args.src, &args.dest, args.quiet).unwrap();
}

fn merge(src: &PathBuf, dest: &PathBuf, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    if !src.is_dir() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!("Source path '{}' is not a directory.", src.display()),
        )));
    }

    if dest.exists() {
        if !dest.is_dir() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                format!("Destination path is not a directory: '{}'", dest.display()),
            )));
        }
    } else {
        fs::create_dir_all(dest)?;
    }

    let files = collect_files(src)?;

    let m = MultiProgress::new();
    let pb_files = m.add(
        ProgressBar::new(files.len() as u64).with_style(
            ProgressStyle::default_bar()
                .template("[{bar:40.cyan/blue}] {pos}/{len} {msg}")?
                .progress_chars("=>-"),
        ),
    );
    if quiet {
        m.set_draw_target(ProgressDrawTarget::hidden());
    }

    for file in files {
        let rel_path = file.strip_prefix(src)?;
        let dest_file = dest.join(rel_path);
        pb_files.set_message(rel_path.display().to_string());

        let parent = dest_file.parent().unwrap();
        fs::create_dir_all(parent)?;

        let src_meta = fs::metadata(&file)?;
        let dest_meta = fs::metadata(parent)?;
        if src_meta.dev() == dest_meta.dev() {
            fs::rename(&file, &dest_file)?;
        } else {
            let pb_bytes = m.add(
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
            move_file_with_progress(&file, &dest_file, &options, progress_handler)?;
            pb_bytes.finish_and_clear();
        }
        pb_files.inc(1);
    }

    fs::remove_dir_all(src)?;

    let elapsed = start.elapsed().as_secs_f32();
    let hours = (elapsed / 3600.0).floor() as u64;
    let minutes = ((elapsed % 3600.0) / 60.0).floor() as u64;
    let seconds = elapsed % 60.0;
    let time_str = match (hours, minutes) {
        (0, 0) => format!("{:.2} seconds", seconds),
        (0, m) => format!("{} minutes {:.2} seconds", m, seconds),
        (h, m) => format!("{} hours {} minutes {:.2} seconds", h, m, seconds),
    };
    pb_files.finish_with_message(format!("finished in {}", time_str));

    Ok(())
}

fn collect_files(dir: &PathBuf) -> std::io::Result<Vec<std::path::PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|path| {
            if path.is_dir() {
                collect_files(&path).unwrap_or_default()
            } else if path.is_file() {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect())
}
