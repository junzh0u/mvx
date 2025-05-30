use anyhow::bail;
use anyhow::ensure;
use colored::Colorize;
use core::panic;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};

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
        merge_directories(src, &dest, mp)?;
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

fn merge_directories(src: &PathBuf, dest: &Path, mp: Option<&MultiProgress>) -> anyhow::Result<()> {
    log::trace!(
        "merge_directories('{}', '{}')",
        src.display(),
        dest.display()
    );
    let files = collect_files(src)?;
    let pb_files = mp.map(|mp| {
        mp.add(
            ProgressBar::new(files.len() as u64).with_style(
                ProgressStyle::with_template("[{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("=>-"),
            ),
        )
    });

    for file in files {
        let rel_path = file.strip_prefix(src)?;
        let dest_file = dest.join(rel_path);
        if let Some(pb) = &pb_files {
            pb.set_message(rel_path.display().to_string());
        }
        file::move_file(&file, &dest_file, mp)?;
        if let Some(pb) = &pb_files {
            pb.inc(1);
        }
    }
    recur_remove_dir(src)?;

    if let Some(pb) = &pb_files {
        pb.finish_and_clear();
        if let Some(mp) = mp {
            mp.remove(pb);
        }
    }

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
