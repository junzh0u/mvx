use crate::MoveOrCopy;
use anyhow::{bail, ensure};
use std::{fs, path::Path};

pub(crate) fn move_or_copy_file<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    mp: Option<&indicatif::MultiProgress>,
    move_or_copy: &MoveOrCopy,
) -> anyhow::Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    log::trace!(
        "move_or_copy_file('{}', '{}', {move_or_copy:?})",
        src.display(),
        dest.display()
    );

    ensure!(src.exists(), "Source '{}' does not exist", src.display());
    ensure!(
        src.is_file(),
        "Source '{}' exists but is not a file",
        src.display()
    );
    ensure!(
        !dest.exists() || dest.is_file(),
        "Destination '{}' already exists and is not a file",
        dest.display()
    );

    if let Some(dest_parent) = dest.parent() {
        fs::create_dir_all(dest_parent)?;
    }

    let result = match move_or_copy {
        MoveOrCopy::Move => fs::rename(src, dest),
        MoveOrCopy::Copy => {
            if dest.exists() {
                fs::remove_file(dest)?;
            }
            reflink::reflink(src, dest)
        }
    };

    match result {
        Ok(()) => {
            let acted = match move_or_copy {
                MoveOrCopy::Move => "Renamed",
                MoveOrCopy::Copy => "Reflinked",
            };
            log::debug!("{acted}: '{}' => '{}'", src.display(), dest.display());
            return Ok(());
        }
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            let fallback = match move_or_copy {
                MoveOrCopy::Move => "copy and delete",
                MoveOrCopy::Copy => "copy",
            };
            log::debug!(
                "'{}' and '{}' are on different devices, falling back to {fallback}.",
                src.display(),
                dest.display()
            );
        }
        Err(e) => {
            println!("Kind: {} Error: {e:?}", e.kind());
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
        match move_or_copy {
            MoveOrCopy::Move => {
                fs_extra::file::move_file_with_progress(src, dest, &copy_options, progress_handler)
            }
            MoveOrCopy::Copy => {
                fs_extra::file::copy_with_progress(src, dest, &copy_options, progress_handler)
            }
        }?;
        pb_bytes.finish_and_clear();
        mp.remove(&pb_bytes);
    } else {
        match move_or_copy {
            MoveOrCopy::Move => fs_extra::file::move_file(src, dest, &copy_options),
            MoveOrCopy::Copy => fs_extra::file::copy(src, dest, &copy_options),
        }?;
    }
    log::debug!("Moved: '{}' => '{}'", src.display(), dest.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{assert_file_copied, assert_file_moved, create_temp_file};
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    fn move_file<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src: Src,
        dest: Dest,
        mp: Option<&indicatif::MultiProgress>,
    ) -> anyhow::Result<()> {
        move_or_copy_file(src, dest, mp, &MoveOrCopy::Move)
    }

    fn copy_file<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src: Src,
        dest: Dest,
        mp: Option<&indicatif::MultiProgress>,
    ) -> anyhow::Result<()> {
        move_or_copy_file(src, dest, mp, &MoveOrCopy::Copy)
    }

    fn assert_error_with_msg(result: anyhow::Result<()>, msg: &str) {
        assert!(result.is_err(), "Expected an error, but got success");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains(msg),
            "Error message doesn't mention that source doesn't exist: {}",
            err_msg
        );
    }

    #[test]
    fn move_file_with_absolute_path() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir.path().join("b");

        move_file(&src_path, &dest_path, None).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn copy_file_with_absolute_path() {
        let work_dir = tempdir().unwrap();
        let src_path = create_temp_file(work_dir.path(), "a", "This is a test file");
        let dest_path = work_dir.path().join("b");

        copy_file(&src_path, &dest_path, None).unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    #[serial]
    fn move_file_with_relative_paths() {
        let work_dir = tempdir().unwrap();
        std::env::set_current_dir(&work_dir).unwrap();
        let src_content = "This is a test file";
        fs::write("a", src_content).unwrap();

        move_file("a", "b", None).unwrap();
        assert_file_moved("a", "b", src_content);
    }

    #[test]
    #[serial]
    fn copy_file_with_relative_paths() {
        let work_dir = tempdir().unwrap();
        std::env::set_current_dir(&work_dir).unwrap();
        let src_content = "This is a test file";
        fs::write("a", src_content).unwrap();

        copy_file("a", "b", None).unwrap();
        assert_file_copied("a", "b");
    }

    #[test]
    fn move_file_overwrites() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = create_temp_file(work_dir.path(), "b", "This is a different file");

        move_file(&src_path, &dest_path, None).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn copy_file_overwrites() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = create_temp_file(work_dir.path(), "b", "This is a different file");

        copy_file(&src_path, &dest_path, None).unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    fn move_file_fails_with_nonexistent_source() {
        let work_dir = tempdir().unwrap();
        let src_path = work_dir.path().join("a");
        let dest_content = "This is a test file";
        let dest_path = create_temp_file(work_dir.path(), "b", dest_content);

        assert!(!src_path.exists(), "Source file should not exist initially");
        assert_error_with_msg(
            move_file(&src_path, "/dest/does/not/matter", None),
            "does not exist",
        );
        assert_eq!(
            fs::read_to_string(dest_path).unwrap(),
            dest_content,
            "Destination file content should remain unchanged"
        );
    }

    #[test]
    fn copy_file_fails_with_nonexistent_source() {
        let work_dir = tempdir().unwrap();
        let src_path = work_dir.path().join("a");
        let dest_content = "This is a test file";
        let dest_path = create_temp_file(work_dir.path(), "b", dest_content);

        assert!(!src_path.exists(), "Source file should not exist initially");
        assert_error_with_msg(
            copy_file(&src_path, "/dest/does/not/matter", None),
            "does not exist",
        );
        assert_eq!(
            fs::read_to_string(dest_path).unwrap(),
            dest_content,
            "Destination file content should remain unchanged"
        );
    }

    #[test]
    fn move_file_creates_intermediate_directories() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir
            .path()
            .join("b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");

        move_file(&src_path, &dest_path, None).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn copy_file_creates_intermediate_directories() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir
            .path()
            .join("b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");

        copy_file(&src_path, &dest_path, None).unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    fn move_file_fails_when_cant_create_intermediate_directories() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        create_temp_file(work_dir.path(), "b/c/d", "");
        let dest_path = work_dir
            .path()
            .join("b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");

        assert_error_with_msg(move_file(&src_path, &dest_path, None), "Not a directory");
        assert!(
            src_path.exists(),
            "Source file should not be moved when error occurs"
        );
        assert!(
            !dest_path.exists(),
            "Destination file should not be created when error occurs"
        );
    }

    #[test]
    fn copy_file_fails_when_cant_create_intermediate_directories() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        create_temp_file(work_dir.path(), "b/c/d", "");
        let dest_path = work_dir
            .path()
            .join("b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");

        assert_error_with_msg(copy_file(&src_path, &dest_path, None), "Not a directory");
        assert!(
            src_path.exists(),
            "Source file should not be moved when error occurs"
        );
        assert!(
            !dest_path.exists(),
            "Destination file should not be created when error occurs"
        );
    }
}
