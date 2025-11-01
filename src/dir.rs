use crate::{MoveOrCopy, bytes_bar_style, new_spinner};
use anyhow::ensure;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn merge_or_copy<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    move_or_copy: &MoveOrCopy,
    mp: Option<&indicatif::MultiProgress>,
) -> anyhow::Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    log::trace!(
        "merge_or_copy('{}', '{}', {move_or_copy:?})",
        src.display(),
        dest.display()
    );

    ensure!(src.exists(), "Source '{}' does not exist", src.display());
    ensure!(
        src.is_dir(),
        "Source '{}' exists but is not a directory",
        src.display()
    );

    if dest.exists() {
        ensure!(
            dest.is_dir(),
            "Destination '{}' already exists and is not a directory",
            dest.display()
        );
    } else {
        fs::create_dir_all(dest)?;
    }

    let mut files = collect_files_in_dir(src)?;
    files.sort_by_key(|p| p.to_string_lossy().to_string());
    let total_size = get_total_size_of_files(&files);

    let pb_total_bytes =
        mp.map(|mp| mp.add(indicatif::ProgressBar::new(total_size).with_style(bytes_bar_style())));
    let pb_files = new_spinner(mp, files.len() as u64);

    for file in files {
        let rel_path = file.strip_prefix(src)?;
        let dest_file = dest.join(rel_path);
        if let Some(pb) = &pb_files {
            pb.inc(1);
            pb.set_message(rel_path.display().to_string());
        }
        crate::file::move_or_copy(
            &file,
            &dest_file,
            move_or_copy,
            mp,
            pb_total_bytes
                .as_ref()
                .map(|pb| {
                    let init_pos = pb.position();
                    move |transit: fs_extra::file::TransitProcess| {
                        pb.set_position(init_pos + transit.copied_bytes);
                    }
                })
                .as_ref(),
        )?;
    }

    match move_or_copy {
        MoveOrCopy::Move => remove_empty_dir(src)?,
        MoveOrCopy::Copy => (),
    }

    if let Some(pb) = &pb_files {
        pb.finish_and_clear();
    }

    Ok(())
}

fn remove_empty_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
    let dir = dir.as_ref();
    log::trace!("remove_empty_dir('{}')", dir.display());
    for entry in fs::read_dir(dir)? {
        remove_empty_dir(entry?.path())?;
    }
    fs::remove_dir(dir)?;
    log::debug!("Removed empty directory: '{}'", dir.display());
    Ok(())
}

fn collect_files_in_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<Vec<PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|path| {
            if path.is_dir() {
                collect_files_in_dir(&path).unwrap_or_default()
            } else if path.is_file() {
                vec![path]
            } else {
                panic!("Unexpected path type: {}", path.display())
            }
        })
        .collect())
}

fn get_total_size_of_files<P: AsRef<Path>>(files: &[P]) -> u64 {
    files
        .iter()
        .filter_map(|f| fs::metadata(f).ok())
        .map(|m| m.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{assert_file_copied, assert_file_moved, create_temp_file};
    use std::collections::HashSet;
    use tempfile::tempdir;

    #[test]
    fn get_total_size_of_files_empty() {
        let files: Vec<std::path::PathBuf> = vec![];
        assert_eq!(get_total_size_of_files(&files), 0);
    }

    #[test]
    fn get_total_size_of_files_single() {
        let temp_dir = tempdir().unwrap();
        let file_contents = [("file1", "hello")];
        let files: Vec<_> = file_contents
            .iter()
            .map(|(file, content)| create_temp_file(temp_dir.path(), file, content))
            .collect();
        let expected_size: u64 = file_contents.iter().map(|(_, c)| c.len() as u64).sum();
        assert_eq!(get_total_size_of_files(&files), expected_size);
    }

    #[test]
    fn get_total_size_of_files_multiple() {
        let temp_dir = tempdir().unwrap();
        let file_contents = [("file1", "abc"), ("file2", "defgh")];
        let files: Vec<_> = file_contents
            .iter()
            .map(|(file, content)| create_temp_file(temp_dir.path(), file, content))
            .collect();
        let expected_size: u64 = file_contents.iter().map(|(_, c)| c.len() as u64).sum();
        assert_eq!(get_total_size_of_files(&files), expected_size);
    }

    #[test]
    fn merge_directory_basic() {
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
        let dest_rel_paths = [
            "file1",
            "file3",
            "subdir/subfile1",
            "subdir/subfile3",
            "subdir/nested/nested_file",
        ];
        for path in dest_rel_paths {
            create_temp_file(dest_dir.path(), path, &format!("From dest: {path}"));
        }

        merge_or_copy(&src_dir, &dest_dir, &MoveOrCopy::Move, None).unwrap();
        for path in src_rel_paths {
            let src_path = src_dir.path().join(path);
            let dest_path = dest_dir.path().join(path);
            assert_file_moved(&src_path, &dest_path, &format!("From source: {path}"));
        }
        for path in dest_rel_paths {
            let dest_path = dest_dir.path().join(path);
            assert!(
                dest_path.exists(),
                "File '{}' should exist",
                dest_path.display()
            );
        }
    }

    #[test]
    fn copy_directory_basic() {
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
        let dest_rel_paths = [
            "file1",
            "file3",
            "subdir/subfile1",
            "subdir/subfile3",
            "subdir/nested/nested_file",
        ];
        for path in dest_rel_paths {
            create_temp_file(dest_dir.path(), path, &format!("From dest: {path}"));
        }

        merge_or_copy(&src_dir, &dest_dir, &MoveOrCopy::Copy, None).unwrap();
        for path in src_rel_paths {
            let src_path = src_dir.path().join(path);
            let dest_path = dest_dir.path().join(path);
            assert_file_copied(&src_path, &dest_path);
        }
        for path in dest_rel_paths {
            let dest_path = dest_dir.path().join(path);
            assert!(
                dest_path.exists(),
                "File '{}' should exist",
                dest_path.display()
            );
        }
    }

    #[test]
    fn collect_files_in_dir_works() {
        let temp_dir = tempdir().unwrap();
        let rel_paths = vec![
            "file1",
            "file2",
            "subdir/subfile1",
            "subdir/subfile2",
            "subdir/nested/nested_file",
        ];
        rel_paths.iter().for_each(|path| {
            create_temp_file(temp_dir.path(), path, "");
        });

        let collected_files: HashSet<PathBuf> = collect_files_in_dir(temp_dir.path())
            .unwrap()
            .into_iter()
            .collect();
        let expected_files: HashSet<PathBuf> = rel_paths
            .into_iter()
            .map(|path| temp_dir.path().join(path))
            .into_iter()
            .collect();
        assert_eq!(collected_files, expected_files);
    }

    #[test]
    fn collect_files_in_empty_dir_works() {
        let temp_dir = tempdir().unwrap();
        assert!(
            collect_files_in_dir(temp_dir.path()).unwrap().is_empty(),
            "Result should be empty for an empty directory"
        );
    }
}
