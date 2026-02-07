use crate::{Ctx, MoveOrCopy, bytes_progress_bar, message_with_arrow};
use anyhow::{bail, ensure};
use colored::Colorize;
use std::{
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

pub(crate) fn move_or_copy<Src: AsRef<Path>, Dest: AsRef<Path>, F: Fn(u64)>(
    src: Src,
    dest: Dest,
    progress_cb: F,
    ctx: &Ctx,
) -> anyhow::Result<String> {
    let src = src.as_ref();
    log::trace!(
        "move_or_copy('{}', '{}', {:?}, force={})",
        src.display(),
        dest.as_ref().display(),
        ctx.moc,
        ctx.force,
    );
    let dest = ensure_dest(src, &dest, ctx.force)?;

    let timer = std::time::Instant::now();
    if let Some(dest_parent) = dest.parent() {
        fs::create_dir_all(dest_parent)?;
    }

    let result = match ctx.moc {
        MoveOrCopy::Move => fs::rename(src, &dest),
        MoveOrCopy::Copy => {
            if dest.exists() {
                fs::remove_file(&dest)?;
            }
            reflink::reflink(src, &dest)
        }
    };
    let fallback = match ctx.moc {
        MoveOrCopy::Move => "copy and delete",
        MoveOrCopy::Copy => "copy",
    };
    match result {
        Ok(()) => {
            return Ok(format!(
                "{} {}: {}",
                "→".green().bold(),
                match ctx.moc {
                    MoveOrCopy::Move => "Renamed",
                    MoveOrCopy::Copy => "Reflinked",
                },
                message_with_arrow(src, dest, ctx.moc)
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            log::debug!(
                "'{}' and '{}' are on different devices, falling back to {fallback}.",
                src.display(),
                dest.display()
            );
        }
        Err(e) if e.raw_os_error().is_some_and(|e| e == libc::ENOTSUP) => {
            log::debug!("Operation not supported, falling back to {fallback}. Full error: {e:?}");
        }
        Err(e) => bail!(e),
    }

    let file_size = fs::metadata(src)?.len();
    let pb_bytes = ctx
        .mp
        .add(bytes_progress_bar(file_size, src, &dest, ctx.moc));

    buffered_copy(src, &dest, &pb_bytes, &progress_cb)?;

    if matches!(ctx.moc, MoveOrCopy::Move) {
        fs::remove_file(src)?;
    }
    pb_bytes.finish_and_clear();

    Ok(format!(
        "{} {} {} in {}: {}",
        "→".green().bold(),
        match ctx.moc {
            MoveOrCopy::Move => "Moved",
            MoveOrCopy::Copy => "Copied",
        },
        indicatif::HumanBytes(file_size),
        indicatif::HumanDuration(timer.elapsed()),
        message_with_arrow(src, dest, ctx.moc)
    ))
}

fn buffered_copy<F: Fn(u64)>(
    src: &Path,
    dest: &Path,
    pb: &indicatif::ProgressBar,
    progress_cb: F,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(fs::File::open(src)?);
    let mut writer = BufWriter::new(fs::File::create(dest)?);
    let mut buf = vec![0u8; 1024 * 1024];
    let mut copied = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        copied += n as u64;
        pb.set_position(copied);
        progress_cb(copied);
    }
    writer.flush()?;
    Ok(())
}

fn ensure_dest<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    force: bool,
) -> anyhow::Result<PathBuf> {
    let src = src.as_ref();
    let mut dest = dest.as_ref().to_path_buf();
    ensure!(src.exists(), "Source '{}' does not exist", src.display());
    ensure!(
        src.is_file(),
        "Source '{}' exists but is not a file",
        src.display()
    );

    if dest.is_dir() || (!dest.exists() && dest.to_string_lossy().ends_with('/')) {
        match src.file_name() {
            Some(name) => dest.push(name),
            None => bail!("Cannot get file name from '{}'", src.display()),
        }
    }
    if dest.exists() {
        ensure!(
            dest.is_file(),
            "Destination '{}' already exists and is not a file",
            dest.display()
        );
        ensure!(
            force,
            "Destination '{}' already exists (use -f to overwrite)",
            dest.display()
        );
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        assert_error_with_msg, assert_file_copied, assert_file_moved, assert_file_not_moved,
        create_temp_file, hidden_multi_progress,
    };
    use serial_test::serial;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    fn move_file<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src: Src,
        dest: Dest,
        force: bool,
    ) -> anyhow::Result<String> {
        let mp = hidden_multi_progress();
        let ctrlc = AtomicBool::new(false);
        let ctx = Ctx {
            moc: MoveOrCopy::Move,
            force,
            dry_run: false,
            mp: &mp,
            ctrlc: &ctrlc,
        };
        move_or_copy(src, dest, |_| {}, &ctx)
    }

    fn copy_file<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src: Src,
        dest: Dest,
        force: bool,
    ) -> anyhow::Result<String> {
        let mp = hidden_multi_progress();
        let ctrlc = AtomicBool::new(false);
        let ctx = Ctx {
            moc: MoveOrCopy::Copy,
            force,
            dry_run: false,
            mp: &mp,
            ctrlc: &ctrlc,
        };
        move_or_copy(src, dest, |_| {}, &ctx)
    }

    #[test]
    fn move_file_succeeds_with_absolute_path() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir.path().join("b");

        move_file(&src_path, &dest_path, false).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn copy_file_succeeds_with_absolute_path() {
        let work_dir = tempdir().unwrap();
        let src_path = create_temp_file(work_dir.path(), "a", "This is a test file");
        let dest_path = work_dir.path().join("b");

        copy_file(&src_path, &dest_path, false).unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    #[serial]
    fn move_file_succeeds_with_relative_path() {
        let work_dir = tempdir().unwrap();
        std::env::set_current_dir(&work_dir).unwrap();
        let src_content = "This is a test file";
        fs::write("a", src_content).unwrap();

        move_file("a", "b", false).unwrap();
        assert_file_moved("a", "b", src_content);
    }

    #[test]
    #[serial]
    fn copy_file_succeeds_with_relative_path() {
        let work_dir = tempdir().unwrap();
        std::env::set_current_dir(&work_dir).unwrap();
        let src_content = "This is a test file";
        fs::write("a", src_content).unwrap();

        copy_file("a", "b", false).unwrap();
        assert_file_copied("a", "b");
    }

    #[test]
    fn move_file_overwrites_existing_dest_with_force() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = create_temp_file(work_dir.path(), "b", "This is a different file");

        move_file(&src_path, &dest_path, true).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn copy_file_overwrites_existing_dest_with_force() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = create_temp_file(work_dir.path(), "b", "This is a different file");

        copy_file(&src_path, &dest_path, true).unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    fn move_file_fails_without_force_when_dest_exists() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = create_temp_file(work_dir.path(), "b", "This is a different file");

        assert_error_with_msg(
            move_file(&src_path, &dest_path, false),
            "already exists (use -f to overwrite)",
        );
    }

    #[test]
    fn copy_file_fails_without_force_when_dest_exists() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = create_temp_file(work_dir.path(), "b", "This is a different file");

        assert_error_with_msg(
            copy_file(&src_path, &dest_path, false),
            "already exists (use -f to overwrite)",
        );
    }

    #[test]
    fn move_file_fails_with_nonexistent_source() {
        let work_dir = tempdir().unwrap();
        let src_path = work_dir.path().join("a");
        let dest_content = "This is a test file";
        let dest_path = create_temp_file(work_dir.path(), "b", dest_content);

        assert!(!src_path.exists(), "Source file should not exist initially");
        assert_error_with_msg(
            move_file(&src_path, "/dest/does/not/matter", false),
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
            copy_file(&src_path, "/dest/does/not/matter", false),
            "does not exist",
        );
        assert_eq!(
            fs::read_to_string(dest_path).unwrap(),
            dest_content,
            "Destination file content should remain unchanged"
        );
    }

    #[test]
    fn move_file_creates_intermediate_directories_automatically() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir
            .path()
            .join("b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");

        move_file(&src_path, &dest_path, false).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn copy_file_creates_intermediate_directories_automatically() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir
            .path()
            .join("b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");

        copy_file(&src_path, &dest_path, false).unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    fn move_file_into_existing_directory() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_dir = work_dir.path().join("dest_dir");
        fs::create_dir_all(&dest_dir).unwrap();

        move_file(&src_path, &dest_dir, false).unwrap();
        assert_file_moved(&src_path, dest_dir.join("a"), src_content);
    }

    #[test]
    fn copy_file_into_existing_directory() {
        let work_dir = tempdir().unwrap();
        let src_path = create_temp_file(work_dir.path(), "a", "content");
        let dest_dir = work_dir.path().join("dest_dir");
        fs::create_dir_all(&dest_dir).unwrap();

        copy_file(&src_path, &dest_dir, false).unwrap();
        assert_file_copied(&src_path, dest_dir.join("a"));
    }

    #[test]
    fn move_fails_when_source_is_directory() {
        let src_dir = tempdir().unwrap();

        assert_error_with_msg(
            move_file(src_dir.path(), "/dest/does/not/matter", false),
            "not a file",
        );
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

        assert_error_with_msg(move_file(&src_path, &dest_path, false), "Not a directory");
        assert_file_not_moved(&src_path, &dest_path);
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

        assert_error_with_msg(copy_file(&src_path, &dest_path, false), "Not a directory");
        assert_file_not_moved(&src_path, &dest_path);
    }
}
