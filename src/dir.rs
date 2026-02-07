use crate::{Ctx, MoveOrCopy, bytes_progress_bar, message_with_arrow};
use anyhow::ensure;
use colored::Colorize;
use std::{fs, path::Path, sync::atomic::Ordering};

pub(crate) fn merge_or_copy<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    ctx: &Ctx,
) -> anyhow::Result<String> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    log::trace!(
        "merge_or_copy('{}', '{}', {:?})",
        src.display(),
        dest.display(),
        ctx.moc,
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

    let timer = std::time::Instant::now();

    let total_size = collect_total_size(src);
    let pb = ctx
        .mp
        .add(bytes_progress_bar(total_size, src, dest, ctx.moc));

    merge_or_copy_recursive(src, dest, ctx, &pb)?;

    if matches!(ctx.moc, MoveOrCopy::Move) {
        let _ = fs::remove_dir(src);
    }
    pb.finish_and_clear();

    Ok(format!(
        "{} {} {} in {}: {}",
        "↣".green().bold(),
        match ctx.moc {
            MoveOrCopy::Move => "Merged",
            MoveOrCopy::Copy => "Copied",
        },
        indicatif::HumanBytes(total_size),
        indicatif::HumanDuration(timer.elapsed()),
        message_with_arrow(src, dest, ctx.moc),
    ))
}

fn merge_or_copy_recursive(
    src: &Path,
    dest: &Path,
    ctx: &Ctx,
    pb: &indicatif::ProgressBar,
) -> anyhow::Result<Vec<String>> {
    let mut entries: Vec<_> = fs::read_dir(src)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();

    let mut msgs = Vec::new();
    for entry in entries {
        let name = entry.file_name().unwrap();
        let dest_entry = dest.join(name);

        if ctx.ctrlc.load(Ordering::Relaxed) {
            for msg in &msgs {
                log::info!("{msg}");
            }
            log::error!(
                "✗ Cancelled: {}",
                message_with_arrow(&entry, &dest_entry, ctx.moc)
            );
            pb.abandon_with_message(format!("✗ {}", pb.message()).red().bold().to_string());
            std::process::exit(130);
        }

        if entry.is_dir() {
            fs::create_dir_all(&dest_entry)?;
            msgs.extend(merge_or_copy_recursive(&entry, &dest_entry, ctx, pb)?);
            if matches!(ctx.moc, MoveOrCopy::Move) {
                let _ = fs::remove_dir(&entry);
            }
        } else {
            let init_pos = pb.position();
            let msg = crate::file::move_or_copy(
                &entry,
                &dest_entry,
                |copied_bytes: u64| {
                    pb.set_position(init_pos + copied_bytes);
                },
                ctx,
            )?;
            msgs.push(msg);
        }
    }
    Ok(msgs)
}

fn collect_total_size(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .map(|path| {
            if path.is_dir() {
                collect_total_size(&path)
            } else {
                fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        assert_file_copied, assert_file_moved, create_temp_file, hidden_multi_progress, noop_ctrlc,
    };
    use tempfile::tempdir;

    fn _merge_or_copy<Src: AsRef<Path>, Dest: AsRef<Path>>(
        src: Src,
        dest: Dest,
        moc: MoveOrCopy,
        force: bool,
    ) -> anyhow::Result<String> {
        let mp = hidden_multi_progress();
        let ctrlc = noop_ctrlc();
        let ctx = Ctx {
            moc,
            force,
            dry_run: false,
            mp: &mp,
            ctrlc: &ctrlc,
        };
        merge_or_copy(src, dest, &ctx)
    }

    #[test]
    fn collect_total_size_empty() {
        let temp_dir = tempdir().unwrap();
        assert_eq!(collect_total_size(temp_dir.path()), 0);
    }

    #[test]
    fn collect_total_size_with_files() {
        let temp_dir = tempdir().unwrap();
        create_temp_file(temp_dir.path(), "file1", "abc");
        create_temp_file(temp_dir.path(), "subdir/file2", "defgh");
        assert_eq!(collect_total_size(temp_dir.path()), 8);
    }

    #[test]
    fn merge_directory_overwrites_existing_files_with_force() {
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

        _merge_or_copy(&src_dir, &dest_dir, MoveOrCopy::Move, true).unwrap();
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
    fn copy_directory_overwrites_existing_files_with_force() {
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

        _merge_or_copy(&src_dir, &dest_dir, MoveOrCopy::Copy, true).unwrap();
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
    fn merge_directory_succeeds_without_force_when_no_files_overlap() {
        let src_dir = tempdir().unwrap();
        let src_rel_paths = ["file1", "subdir/subfile1"];
        for path in src_rel_paths {
            create_temp_file(src_dir.path(), path, &format!("From source: {path}"));
        }

        let dest_dir = tempdir().unwrap();
        let dest_rel_paths = ["file2", "subdir/subfile2"];
        for path in dest_rel_paths {
            create_temp_file(dest_dir.path(), path, &format!("From dest: {path}"));
        }

        // force=false should work because no files overlap
        _merge_or_copy(&src_dir, &dest_dir, MoveOrCopy::Move, false).unwrap();

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
    fn merge_directory_fails_without_force_when_files_overlap() {
        let src_dir = tempdir().unwrap();
        create_temp_file(src_dir.path(), "file1", "From source");

        let dest_dir = tempdir().unwrap();
        create_temp_file(dest_dir.path(), "file1", "From dest");

        // force=false should fail because file1 exists in both
        let result = _merge_or_copy(&src_dir, &dest_dir, MoveOrCopy::Move, false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn merge_preserves_empty_directories() {
        let src_dir = tempdir().unwrap();
        create_temp_file(src_dir.path(), "file1", "content");
        fs::create_dir_all(src_dir.path().join("empty_dir")).unwrap();
        fs::create_dir_all(src_dir.path().join("subdir/empty_nested")).unwrap();

        let dest_dir = tempdir().unwrap();
        _merge_or_copy(&src_dir, &dest_dir, MoveOrCopy::Move, false).unwrap();

        assert!(dest_dir.path().join("empty_dir").is_dir());
        assert!(dest_dir.path().join("subdir/empty_nested").is_dir());
        assert!(!src_dir.path().exists());
    }

    #[test]
    fn copy_preserves_empty_directories() {
        let src_dir = tempdir().unwrap();
        create_temp_file(src_dir.path(), "file1", "content");
        fs::create_dir_all(src_dir.path().join("empty_dir")).unwrap();

        let dest_dir = tempdir().unwrap();
        _merge_or_copy(&src_dir, &dest_dir, MoveOrCopy::Copy, false).unwrap();

        assert!(dest_dir.path().join("empty_dir").is_dir());
        assert!(src_dir.path().join("empty_dir").is_dir());
    }
}
