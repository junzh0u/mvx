use anyhow::bail;
use anyhow::ensure;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use colored::Colorize;
use core::panic;
use fs_extra::file::{CopyOptions, TransitProcess, move_file_with_progress};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::fs;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,

    /// Path to move from
    src: PathBuf,

    /// Path to move or merge to
    dest: PathBuf,
}

/// # Errors
///
/// Will return `Err` if move/merge fails for any reason.
pub fn run(cli: &Cli) -> anyhow::Result<()> {
    let mp = MultiProgress::new();
    if cli.verbosity.log_level() < Some(log::Level::Info) {
        mp.set_draw_target(ProgressDrawTarget::hidden());
    }
    let mp_clone = mp.clone();

    let _ = env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .format(move |buf, record| {
            let ts = chrono::Local::now().to_rfc3339().bold();
            let file_and_line = format!(
                "[{:>10}:{:<3}]",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
            )
            .italic();
            let level = match record.level() {
                log::Level::Error => "ERROR".red(),
                log::Level::Warn => "WARN ".yellow(),
                log::Level::Info => "INFO ".green(),
                log::Level::Debug => "DEBUG".blue(),
                log::Level::Trace => "TRACE".magenta(),
            }
            .bold();

            let msg = format!("{ts} {file_and_line} {level} {}", record.args());

            if mp_clone.is_hidden() {
                writeln!(buf, "{msg}")
            } else {
                mp_clone.println(msg)
            }
        })
        .try_init();

    log::trace!("run({cli:?})");
    let start = std::time::Instant::now();
    let pb_info = mp.add(
        ProgressBar::new_spinner()
            .with_style(ProgressStyle::default_spinner().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")),
    );
    if !mp.is_hidden() {
        pb_info.enable_steady_tick(std::time::Duration::from_millis(100));
    }

    let src = &cli.src;
    let mut dest = cli.dest.clone();
    ensure!(
        src.exists(),
        "Source path '{}' does not exist.",
        src.display()
    );
    if src.is_file() {
        if dest.is_dir() || (!dest.exists() && dest.to_string_lossy().ends_with('/')) {
            match src.file_name() {
                Some(name) => dest.push(name),
                None => bail!("Cannot get file name from '{}'", src.display()),
            }
        }
        pb_info.set_message(format!(
            "Moving: '{}' => '{}'",
            src.display(),
            dest.display(),
        ));
        move_file(src, &dest, &mp)?;
        pb_info.set_style(ProgressStyle::with_template("{msg}")?);
        pb_info.finish_with_message(format!(
            "{} Moved in {}: '{}' => '{}'",
            "→".green().bold(),
            HumanDuration(start.elapsed()),
            src.display(),
            dest.display(),
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
            "Merging: '{}' => '{}'",
            src.display(),
            dest.display(),
        ));
        merge_directories(src, &dest, &mp)?;
        pb_info.set_style(ProgressStyle::with_template("{msg}")?);
        pb_info.finish_with_message(format!(
            "{} Merged in {}: '{}' => '{}'",
            "↣".green().bold(),
            HumanDuration(start.elapsed()),
            src.display(),
            dest.display(),
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
    log::trace!(
        "merge_directories('{}', '{}')",
        src.display(),
        dest.display()
    );
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
    recur_remove_dir(src)?;

    pb_files.finish_and_clear();
    mp.remove(&pb_files);

    Ok(())
}

fn recur_remove_dir(dir: &PathBuf) -> std::io::Result<()> {
    log::trace!("recur_remove_dir('{}')", dir.display());
    for entry in fs::read_dir(dir)? {
        recur_remove_dir(&entry?.path())?;
    }
    fs::remove_dir(dir)?;
    log::debug!("Removed empty directory: '{}'", dir.display());
    Ok(())
}

fn move_file(src: &PathBuf, dest: &PathBuf, mp: &MultiProgress) -> anyhow::Result<()> {
    log::trace!("move_file('{}', '{}')", src.display(), dest.display());
    let src_meta = fs::metadata(src)?;

    let dest_abs = std::path::absolute(dest)?;
    if let Some(dest_parent) = dest_abs.parent() {
        fs::create_dir_all(dest_parent)?;
        if src_meta.dev() == fs::metadata(dest_parent)?.dev() {
            fs::rename(src, dest)?;
            log::debug!("Renamed: '{}' => '{}'", src.display(), dest.display());
            return Ok(());
        }
    }

    let pb_bytes = mp.add(
        ProgressBar::new(src_meta.len()).with_style(
            ProgressStyle::with_template(
                "[{bar:40.green/white}] {bytes}/{total_bytes} [{bytes_per_sec}] (ETA: {eta})",
            )?
            .progress_chars("=>-"),
        ),
    );
    let progress_handler = |transit: TransitProcess| {
        pb_bytes.set_position(transit.copied_bytes);
    };
    let options = CopyOptions::new().overwrite(true);
    move_file_with_progress(src, dest, &options, progress_handler)?;
    log::debug!("Moved: '{}' => '{}'", src.display(), dest.display());
    pb_bytes.finish_and_clear();
    mp.remove(&pb_bytes);
    Ok(())
}

fn collect_files(dir: &PathBuf) -> std::io::Result<Vec<PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|path| {
            if path.is_dir() {
                collect_files(&path).unwrap_or_default()
            } else if path.is_file() {
                vec![path]
            } else {
                panic!("Unexpected path type: {}", path.display())
            }
        })
        .collect())
}
