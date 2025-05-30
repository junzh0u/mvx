use anyhow::bail;
use anyhow::ensure;
use colored::Colorize;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};

mod dir;
mod file;

/// # Errors
///
/// Will return `Err` if move/merge fails for any reason.
pub fn run(src: &PathBuf, dest: &Path, mp: Option<&MultiProgress>) -> anyhow::Result<()> {
    log::trace!(
        "run('{}', '{}', {:?})",
        src.display(),
        dest.display(),
        mp.map(|_| "MultiProgress"),
    );
    let start = std::time::Instant::now();
    let pb_info = mp.map(|mp| {
        let pb = mp.add(
            ProgressBar::new_spinner()
                .with_style(ProgressStyle::default_spinner().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    });

    let mut dest = dest.to_path_buf();
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
        if let Some(pb) = &pb_info {
            pb.set_message(format!(
                "Moving: '{}' => '{}'",
                src.display(),
                dest.display(),
            ));
        }
        file::move_file(src, &dest, mp)?;
        if let Some(pb) = &pb_info {
            pb.set_style(ProgressStyle::with_template("{msg}")?);
            pb.finish_with_message(format!(
                "{} Moved in {}: '{}' => '{}'",
                "→".green().bold(),
                HumanDuration(start.elapsed()),
                src.display(),
                dest.display(),
            ));
        }
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
        if let Some(pb) = &pb_info {
            pb.set_message(format!(
                "Merging: '{}' => '{}'",
                src.display(),
                dest.display(),
            ));
        }
        dir::merge_directories(src, &dest, mp)?;
        if let Some(pb) = &pb_info {
            pb.set_style(ProgressStyle::with_template("{msg}")?);
            pb.finish_with_message(format!(
                "{} Merged in {}: '{}' => '{}'",
                "↣".green().bold(),
                HumanDuration(start.elapsed()),
                src.display(),
                dest.display(),
            ));
        }
    } else {
        bail!(
            "Source path is neither a file nor directory: '{}'",
            src.display()
        )
    }
    Ok(())
}
