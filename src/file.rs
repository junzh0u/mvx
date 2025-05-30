use std::{fs, path::PathBuf};

use anyhow::bail;

pub fn move_file(
    src: &PathBuf,
    dest: &PathBuf,
    mp: Option<&indicatif::MultiProgress>,
) -> anyhow::Result<()> {
    log::trace!("move_file('{}', '{}')", src.display(), dest.display());
    if let Some(dest_parent) = dest.parent() {
        fs::create_dir_all(dest_parent)?;
    }

    match fs::rename(src, dest) {
        Ok(()) => {
            log::debug!("Renamed: '{}' => '{}'", src.display(), dest.display());
            return Ok(());
        }
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            log::debug!(
                "'{}' and '{}' are on different devices, falling back to copy and delete.",
                src.display(),
                dest.display()
            );
        }
        Err(e) => {
            bail!(e);
        }
    }

    let copy_options = fs_extra::file::CopyOptions::new().overwrite(true);
    if let Some(mp) = mp {
        let pb_bytes = mp.add(
            indicatif::ProgressBar::new(fs::metadata(src)?.len()).with_style(
                indicatif::ProgressStyle::with_template(
                    "[{bar:40.green/white}] {bytes}/{total_bytes} [{bytes_per_sec}] (ETA: {eta})",
                )?
                .progress_chars("=>-"),
            ),
        );
        let progress_handler = |transit: fs_extra::file::TransitProcess| {
            pb_bytes.set_position(transit.copied_bytes);
        };
        fs_extra::file::move_file_with_progress(src, dest, &copy_options, progress_handler)?;
        pb_bytes.finish_and_clear();
        mp.remove(&pb_bytes);
    } else {
        fs_extra::file::move_file(src, dest, &copy_options)?;
    }
    log::debug!("Moved: '{}' => '{}'", src.display(), dest.display());
    Ok(())
}
