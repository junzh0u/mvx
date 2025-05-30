use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::ensure;

pub(crate) fn merge_directories<Src: AsRef<Path>, Dest: AsRef<Path>>(
    src: Src,
    dest: Dest,
    mp: Option<&indicatif::MultiProgress>,
) -> anyhow::Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    ensure!(src.exists(), "Source '{}' does not exist", src.display());
    ensure!(
        src.is_dir(),
        "Source '{}' exists but is not a directory",
        src.display()
    );
    ensure!(
        !dest.exists() || dest.is_dir(),
        "Destination '{}' already exists and is not a directory",
        dest.display()
    );

    log::trace!(
        "merge_directories('{}', '{}')",
        src.display(),
        dest.display()
    );
    let files = collect_files_in_dir(src)?;
    let pb_files = mp.map(|mp| {
        mp.add(
            indicatif::ProgressBar::new(files.len() as u64).with_style(
                indicatif::ProgressStyle::with_template("[{bar:40.cyan/blue}] {pos}/{len} {msg}")
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
        crate::file::move_file(&file, &dest_file, mp)?;
        if let Some(pb) = &pb_files {
            pb.inc(1);
        }
    }
    remove_empty_dir(src)?;

    if let Some(pb) = &pb_files {
        pb.finish_and_clear();
        if let Some(mp) = mp {
            mp.remove(pb);
        }
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
