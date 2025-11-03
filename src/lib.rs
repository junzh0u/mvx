use anyhow::ensure;
use colored::Colorize;
use log::LevelFilter;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

mod dir;
mod file;

#[derive(Debug)]
pub enum MoveOrCopy {
    Move,
    Copy,
}

pub fn init_logging(level_filter: LevelFilter) -> Option<indicatif::MultiProgress> {
    let mp = (level_filter >= LevelFilter::Info).then(indicatif::MultiProgress::new);
    let mp_clone = mp.clone();

    env_logger::Builder::new()
        .filter_level(level_filter)
        .format(move |buf, record| {
            let ts = chrono::Local::now().to_rfc3339().bold();

            let file_and_line = format!(
                "[{}:{}]",
                record
                    .file()
                    .map(Path::new)
                    .and_then(Path::file_name)
                    .unwrap_or_default()
                    .display(),
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

            let msg = format!("{ts} {file_and_line:12} {level} {}", record.args());

            match &mp_clone {
                Some(mp) => mp.println(msg),
                None => writeln!(buf, "{msg}"),
            }
        })
        .init();

    mp
}

/// # Errors
///
/// Will return `Err` if move/merge fails for any reason.
pub fn run_batch<Src: AsRef<Path>, Srcs: AsRef<[Src]>, Dest: AsRef<Path>>(
    srcs: Srcs,
    dest: Dest,
    move_or_copy: &MoveOrCopy,
    mp: Option<&indicatif::MultiProgress>,
    ctrlc: &Receiver<()>,
) -> anyhow::Result<String> {
    let srcs = srcs.as_ref();
    let dest = dest.as_ref();
    log::trace!(
        "run_batch('{:?}', '{}', {:?}, {:?})",
        srcs.iter()
            .map(|s| s.as_ref().display())
            .collect::<Vec<_>>(),
        dest.display(),
        mp.map(|_| "MultiProgress"),
        move_or_copy,
    );

    if srcs.len() > 1 {
        ensure!(
            dest.is_dir(),
            "When copying multiple sources, the destination must be a directory.",
        );
    }

    for src in srcs {
        if ctrlc.try_recv().is_ok() {
            log::error!(
                "✗ Cancelled: {}",
                message_with_arrow(src, dest, move_or_copy)
            );
            std::process::exit(130);
        }

        let src = src.as_ref();
        ensure!(
            src.is_file() || src.is_dir(),
            "Source path '{}' is neither a file nor directory.",
            src.display()
        );

        let msg = if src.is_file() {
            file::move_or_copy(src, dest, move_or_copy, mp, None::<&fn(_)>)?
        } else {
            dir::merge_or_copy(src, dest, move_or_copy, mp, ctrlc)?
        };
        println!("{msg}");
    }

    Ok(String::new())
}

/// # Errors
///
/// Will return `Err` if can not register Ctrl-C handler.
pub fn ctrlc_channel() -> anyhow::Result<Receiver<()>> {
    let (tx, rx) = channel();
    ctrlc::set_handler(move || {
        log::warn!("✗ Ctrl-C detected, cancelling...");
        let _ = tx.send(());
    })?;

    Ok(rx)
}

fn bytes_progress_bar<Src: AsRef<Path>, Dest: AsRef<Path>>(
    size: u64,
    src: Src,
    dest: Dest,
    move_or_copy: &MoveOrCopy,
) -> indicatif::ProgressBar {
    let style = indicatif::ProgressStyle::with_template(
        "{total_bytes:>11} [{bar:40.green/white}] {bytes:<11} ({bytes_per_sec:>13}, ETA: {eta_precise} ) {msg}",
    ).unwrap().progress_chars(
        match move_or_copy {
            MoveOrCopy::Move => "->-",
            MoveOrCopy::Copy => "=>=",
        }
    );
    indicatif::ProgressBar::new(size)
        .with_style(style)
        .with_message(message_with_arrow(src, dest, move_or_copy))
}

fn message_with_arrow<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    move_or_copy: &MoveOrCopy,
) -> String {
    format!(
        "{} {} {}",
        src.as_ref().display(),
        match move_or_copy {
            MoveOrCopy::Move => "->",
            MoveOrCopy::Copy => "=>",
        },
        dest.as_ref().display()
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    pub(crate) fn noop_receiver() -> Receiver<()> {
        let (tx, rx) = channel();
        drop(tx);
        rx
    }

    pub(crate) fn create_temp_file<P: AsRef<Path>>(dir: P, name: &str, content: &str) -> PathBuf {
        let path = dir.as_ref().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        path
    }

    pub(crate) fn assert_file_moved<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src_path: Src,
        dest_path: Dest,
        expected_content: &str,
    ) {
        let src = src_path.as_ref();
        let dest = dest_path.as_ref();
        assert!(
            !src.exists(),
            "Source file still exists at {}",
            src.display()
        );
        assert!(
            dest.exists(),
            "Destination file does not exist at {}",
            dest.display()
        );
        let moved_content = fs::read_to_string(dest_path).unwrap();
        assert_eq!(
            moved_content, expected_content,
            "File content doesn't match after move"
        );
    }

    pub(crate) fn assert_file_not_moved<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src_path: Src,
        dest_path: Dest,
    ) {
        let src = src_path.as_ref();
        let dest = dest_path.as_ref();
        assert!(
            src.exists(),
            "Source file does not exist at {}",
            src.display()
        );
        assert!(
            !dest.exists(),
            "Destination file should not exist at {}",
            dest.display()
        );
    }

    pub(crate) fn assert_file_copied<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src_path: Src,
        dest_path: Dest,
    ) {
        let src = src_path.as_ref();
        let dest = dest_path.as_ref();
        assert!(
            src.exists(),
            "Source file does not exists at {}",
            src.display()
        );
        assert!(
            dest.exists(),
            "Destination file does not exist at {}",
            dest.display()
        );
        assert_eq!(
            fs::read_to_string(src).unwrap(),
            fs::read_to_string(dest_path).unwrap(),
            "File content doesn't match after copy"
        );
    }

    pub(crate) fn assert_error_with_msg(result: anyhow::Result<String>, msg: &str) {
        assert!(result.is_err(), "Expected an error, but got success");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains(msg),
            "Error message doesn't mention that source doesn't exist: {}",
            err_msg
        );
    }

    #[test]
    fn move_file_basic() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir.path().join("b");

        run_batch(
            [&src_path],
            &dest_path,
            &MoveOrCopy::Move,
            None,
            &noop_receiver(),
        )
        .unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn move_multiple_files() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_paths = vec![
            create_temp_file(work_dir.path(), "a", src_content),
            create_temp_file(work_dir.path(), "b", src_content),
        ];
        let dest_dir = work_dir.path().join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        run_batch(
            &src_paths,
            &dest_dir,
            &MoveOrCopy::Move,
            None,
            &noop_receiver(),
        )
        .unwrap();
        for src_path in src_paths {
            let dest_path = dest_dir.join(src_path.file_name().unwrap());
            assert_file_moved(&src_path, &dest_path, src_content);
        }
    }

    #[test]
    fn move_multiple_files_fails_if_dest_not_dir() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_paths = vec![
            create_temp_file(work_dir.path(), "a", src_content),
            create_temp_file(work_dir.path(), "b", src_content),
        ];
        let dest_dir = work_dir.path().join("dest");
        // fs::create_dir_all(&dest_dir).unwrap();

        assert_error_with_msg(
            run_batch(
                &src_paths,
                &dest_dir,
                &MoveOrCopy::Move,
                None,
                &noop_receiver(),
            ),
            "When copying multiple sources, the destination must be a directory.",
        );
        for src_path in src_paths {
            let dest_path = dest_dir.join(src_path.file_name().unwrap());
            assert_file_not_moved(&src_path, &dest_path);
        }
    }

    #[test]
    fn copy_file_basic() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir.path().join("b");

        run_batch(
            [&src_path],
            &dest_path,
            &MoveOrCopy::Copy,
            None,
            &noop_receiver(),
        )
        .unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    fn move_file_to_directory() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_name = "a";
        let src_path = create_temp_file(&work_dir, src_name, src_content);
        let dest_dir = work_dir.path().join("b/c/");

        run_batch(
            [&src_path],
            &dest_dir,
            &MoveOrCopy::Move,
            None,
            &noop_receiver(),
        )
        .unwrap();
        assert_file_moved(src_path, dest_dir.join(src_name), src_content);
    }

    #[test]
    fn copy_file_to_directory() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_name = "a";
        let src_path = create_temp_file(&work_dir, src_name, src_content);
        let dest_dir = work_dir.path().join("b/c/");

        run_batch(
            [&src_path],
            &dest_dir,
            &MoveOrCopy::Copy,
            None,
            &noop_receiver(),
        )
        .unwrap();
        assert_file_copied(src_path, dest_dir.join(src_name));
    }

    #[test]
    fn merge_directories_basic() {
        let src_dir = tempdir().unwrap();
        let src_rel_paths = [
            "file1",
            "file2",
            "subdir/subfile1",
            "subdir/subfile2",
            "subdir/nested/nested_file",
        ];
        for path in src_rel_paths {
            create_temp_file(src_dir.path(), path, &format!("From source: {path}"));
        }

        let dest_dir = tempdir().unwrap();
        run_batch(
            [&src_dir],
            &dest_dir,
            &MoveOrCopy::Move,
            None,
            &noop_receiver(),
        )
        .unwrap();
        for path in src_rel_paths {
            let src_path = src_dir.path().join(path);
            let dest_path = dest_dir.path().join(path);
            assert_file_moved(&src_path, &dest_path, &format!("From source: {path}"));
        }
    }

    #[test]
    fn merge_multiple_directories() {
        let src_num = 5;
        let src_dirs = (0..src_num)
            .filter_map(|_| tempdir().ok())
            .collect::<Vec<tempfile::TempDir>>();
        let src_rel_paths = (0..src_num)
            .map(|i| format! {"nested{i}/file{i}"})
            .collect::<Vec<String>>();
        (0..src_num).for_each(|i| {
            create_temp_file(&src_dirs[i], &src_rel_paths[i], &format!("content{i}"));
        });

        let dest_dir = tempdir().unwrap();
        run_batch(
            &src_dirs,
            &dest_dir,
            &MoveOrCopy::Move,
            None,
            &noop_receiver(),
        )
        .unwrap();
        (0..src_num).for_each(|i| {
            let src_path = src_dirs[i].path().join(&src_rel_paths[i]);
            let dest_path = dest_dir.path().join(&src_rel_paths[i]);
            assert_file_moved(&src_path, &dest_path, &format!("content{i}"));
        });
    }

    #[test]
    fn copy_directories_basic() {
        let src_dir = tempdir().unwrap();
        let src_rel_paths = [
            "file1",
            "file2",
            "subdir/subfile1",
            "subdir/subfile2",
            "subdir/nested/nested_file",
        ];
        for path in src_rel_paths {
            create_temp_file(src_dir.path(), path, &format!("From source: {path}"));
        }

        let dest_dir = tempdir().unwrap();
        run_batch(
            [&src_dir],
            &dest_dir,
            &MoveOrCopy::Copy,
            None,
            &noop_receiver(),
        )
        .unwrap();
        for path in src_rel_paths {
            let src_path = src_dir.path().join(path);
            let dest_path = dest_dir.path().join(path);
            assert_file_copied(&src_path, &dest_path);
        }
    }
}
