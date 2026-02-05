use anyhow::{bail, ensure};
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

impl MoveOrCopy {
    #[must_use]
    pub const fn verb(&self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Copy => "copy",
        }
    }

    #[must_use]
    pub const fn arrow(&self) -> &'static str {
        match self {
            Self::Move => "->",
            Self::Copy => "=>",
        }
    }

    #[must_use]
    pub const fn progress_chars(&self) -> &'static str {
        match self {
            Self::Move => "->-",
            Self::Copy => "=>=",
        }
    }
}

#[must_use]
pub fn init_logging(level_filter: LevelFilter) -> indicatif::MultiProgress {
    let mp = indicatif::MultiProgress::new();
    if level_filter < LevelFilter::Info {
        mp.set_draw_target(indicatif::ProgressDrawTarget::hidden());
    }
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
            if mp_clone.is_hidden() {
                writeln!(buf, "{msg}")
            } else {
                mp_clone.println(msg)
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
    moc: &MoveOrCopy,
    force: bool,
    dry_run: bool,
    mp: &indicatif::MultiProgress,
    ctrlc: &Receiver<()>,
) -> anyhow::Result<String> {
    let srcs = srcs
        .as_ref()
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect::<Vec<_>>();
    let dest = dest.as_ref();
    log::trace!(
        "run_batch('{:?}', '{}', {:?})",
        srcs.iter().map(|s| s.display()).collect::<Vec<_>>(),
        dest.display(),
        moc,
    );

    let mut all_files = true;
    let mut all_dirs = true;
    for src in &srcs {
        if src.is_file() {
            all_dirs = false;
        } else if src.is_dir() {
            all_files = false;
        } else {
            bail!(
                "Source path '{}' is neither a file nor directory.",
                src.display()
            );
        }
    }

    if srcs.len() > 1 {
        ensure!(
            dest.is_dir(),
            "When there are multiple sources, the destination must be a directory.",
        );
        ensure!(
            all_files || all_dirs,
            "When there are multiple sources, they must be all files or all directories.",
        );
    }

    if dry_run {
        for src in srcs {
            println!(
                "Would {} '{}' to '{}'",
                moc.verb(),
                src.display(),
                dest.display()
            );
        }
        return Ok(String::new());
    }

    let spinner = new_spinner(mp, srcs.len() as u64);
    for src in srcs {
        if ctrlc.try_recv().is_ok() {
            log::error!("✗ Cancelled: {}", message_with_arrow(src, dest, moc));
            std::process::exit(130);
        }

        spinner.set_message(src.display().to_string());
        spinner.inc(1);

        let msg = if src.is_file() {
            file::move_or_copy(src, dest, moc, force, mp, |_| {})?
        } else {
            dir::merge_or_copy(src, dest, moc, force, mp, ctrlc)?
        };
        mp.println(msg)?;
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

fn new_spinner(mp: &indicatif::MultiProgress, len: u64) -> indicatif::ProgressBar {
    let style = indicatif::ProgressStyle::with_template("{spinner:.blue} {pos:>4}/{len:<4} {msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    let pb = mp.add(indicatif::ProgressBar::new(len)).with_style(style);
    if len > 1 {
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
    } else {
        pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
    }
    pb
}

fn bytes_progress_bar<Src: AsRef<Path>, Dest: AsRef<Path>>(
    size: u64,
    src: Src,
    dest: Dest,
    moc: &MoveOrCopy,
) -> indicatif::ProgressBar {
    let template = if src.as_ref().is_dir() {
        "{total_bytes:>11} [{bar:40.cyan/white}] {bytes:<11} ({bytes_per_sec:>13}, ETA: {eta_precise} ) {msg}"
    } else {
        "{total_bytes:>11} [{bar:40.green/white}] {bytes:<11} ({bytes_per_sec:>13}, ETA: {eta_precise} ) {msg}"
    };
    let style = indicatif::ProgressStyle::with_template(template)
        .unwrap()
        .progress_chars(moc.progress_chars());
    indicatif::ProgressBar::new(size)
        .with_style(style)
        .with_message(message_with_arrow(src, dest, moc))
}

fn message_with_arrow<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    moc: &MoveOrCopy,
) -> String {
    format!(
        "{} {} {}",
        src.as_ref().display(),
        moc.arrow(),
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

    pub(crate) fn hidden_multi_progress() -> indicatif::MultiProgress {
        indicatif::MultiProgress::with_draw_target(indicatif::ProgressDrawTarget::hidden())
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

    fn _run_batch<Src: AsRef<Path>, Srcs: AsRef<[Src]>, Dest: AsRef<Path>>(
        srcs: Srcs,
        dest: Dest,
        moc: &MoveOrCopy,
        force: bool,
    ) -> anyhow::Result<String> {
        run_batch(
            srcs,
            dest,
            moc,
            force,
            false,
            &hidden_multi_progress(),
            &noop_receiver(),
        )
    }

    #[test]
    fn move_file_to_new_dest() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir.path().join("b");

        _run_batch([&src_path], &dest_path, &MoveOrCopy::Move, false).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn move_multiple_files_to_directory() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_paths = vec![
            create_temp_file(work_dir.path(), "a", src_content),
            create_temp_file(work_dir.path(), "b", src_content),
        ];
        let dest_dir = work_dir.path().join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        _run_batch(&src_paths, &dest_dir, &MoveOrCopy::Move, false).unwrap();
        for src_path in src_paths {
            let dest_path = dest_dir.join(src_path.file_name().unwrap());
            assert_file_moved(&src_path, &dest_path, src_content);
        }
    }

    #[test]
    fn move_multiple_files_fails_when_dest_is_not_directory() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_paths = vec![
            create_temp_file(work_dir.path(), "a", src_content),
            create_temp_file(work_dir.path(), "b", src_content),
        ];
        let dest_dir = work_dir.path().join("dest");

        assert_error_with_msg(
            _run_batch(&src_paths, &dest_dir, &MoveOrCopy::Move, false),
            "When there are multiple sources, the destination must be a directory.",
        );
        for src_path in src_paths {
            let dest_path = dest_dir.join(src_path.file_name().unwrap());
            assert_file_not_moved(&src_path, &dest_path);
        }
    }

    #[test]
    fn move_mix_of_files_and_directories_fails() {
        let work_dir = tempdir().unwrap();
        let src_dir = tempdir().unwrap();
        let src_paths = vec![
            create_temp_file(work_dir.path(), "a", "This is a test file"),
            src_dir.path().to_path_buf(),
        ];
        let dest_dir = work_dir.path().join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        assert_error_with_msg(
            _run_batch(&src_paths, &dest_dir, &MoveOrCopy::Move, false),
            "When there are multiple sources, they must be all files or all directories.",
        );
    }

    #[test]
    fn copy_file_basic() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir.path().join("b");

        _run_batch([&src_path], &dest_path, &MoveOrCopy::Copy, false).unwrap();
        assert_file_copied(&src_path, &dest_path);
    }

    #[test]
    fn move_file_into_directory_with_trailing_slash() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_name = "a";
        let src_path = create_temp_file(&work_dir, src_name, src_content);
        let dest_dir = work_dir.path().join("b/c/");

        _run_batch([&src_path], &dest_dir, &MoveOrCopy::Move, false).unwrap();
        assert_file_moved(src_path, dest_dir.join(src_name), src_content);
    }

    #[test]
    fn copy_file_into_directory_with_trailing_slash() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_name = "a";
        let src_path = create_temp_file(&work_dir, src_name, src_content);
        let dest_dir = work_dir.path().join("b/c/");

        _run_batch([&src_path], &dest_dir, &MoveOrCopy::Copy, false).unwrap();
        assert_file_copied(src_path, dest_dir.join(src_name));
    }

    #[test]
    fn merge_directory_into_empty_dest() {
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
        _run_batch([&src_dir], &dest_dir, &MoveOrCopy::Move, false).unwrap();
        for path in src_rel_paths {
            let src_path = src_dir.path().join(path);
            let dest_path = dest_dir.path().join(path);
            assert_file_moved(&src_path, &dest_path, &format!("From source: {path}"));
        }
    }

    #[test]
    fn merge_multiple_directories_into_dest() {
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
        _run_batch(&src_dirs, &dest_dir, &MoveOrCopy::Move, false).unwrap();
        (0..src_num).for_each(|i| {
            let src_path = src_dirs[i].path().join(&src_rel_paths[i]);
            let dest_path = dest_dir.path().join(&src_rel_paths[i]);
            assert_file_moved(&src_path, &dest_path, &format!("content{i}"));
        });
    }

    #[test]
    fn copy_directory_into_empty_dest() {
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
        _run_batch([&src_dir], &dest_dir, &MoveOrCopy::Copy, false).unwrap();
        for path in src_rel_paths {
            let src_path = src_dir.path().join(path);
            let dest_path = dest_dir.path().join(path);
            assert_file_copied(&src_path, &dest_path);
        }
    }
}
