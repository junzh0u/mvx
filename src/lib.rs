use anyhow::bail;
use anyhow::ensure;
use colored::Colorize;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;

mod dir;
mod file;

/// # Errors
///
/// Will return `Err` if move/merge fails for any reason.
pub fn run<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    mp: Option<&MultiProgress>,
) -> anyhow::Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();
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

    ensure!(
        src.exists(),
        "Source path '{}' does not exist.",
        src.display()
    );
    if src.is_file() {
        let mut dest = dest.to_path_buf();
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
            fs::create_dir_all(dest)?;
        }
        if let Some(pb) = &pb_info {
            pb.set_message(format!(
                "Merging: '{}' => '{}'",
                src.display(),
                dest.display(),
            ));
        }
        dir::merge_directories(src, dest, mp)?;
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

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

    #[test]
    fn move_file() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_path = create_temp_file(work_dir.path(), "a", src_content);
        let dest_path = work_dir.path().join("b");

        run(&src_path, &dest_path, None).unwrap();
        assert_file_moved(&src_path, &dest_path, src_content);
    }

    #[test]
    fn move_file_to_directory() {
        let work_dir = tempdir().unwrap();
        let src_content = "This is a test file";
        let src_name = "a";
        let src_path = create_temp_file(&work_dir, src_name, src_content);
        let dest_dir = work_dir.path().join("b/c/");

        run(&src_path, &dest_dir, None).unwrap();
        assert_file_moved(src_path, dest_dir.join(src_name), src_content);
    }

    #[test]
    fn merge_directories() {
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
        run(&src_dir, &dest_dir, None).unwrap();
        for path in src_rel_paths {
            let src_path = src_dir.path().join(path);
            let dest_path = dest_dir.path().join(path);
            assert_file_moved(&src_path, &dest_path, &format!("From source: {path}"));
        }
    }
}
