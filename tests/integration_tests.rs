use anyhow::Result;
use mvx::run;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, tempdir};

// Helper function to create a temporary file with content
fn create_temp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

// Helper function to run mvx command with quiet mode
fn run_mvx(src: &PathBuf, dest: &str) -> Result<()> {
    run(src, Path::new(dest), None)
}

// Helper function to verify a file was moved correctly
fn verify_file_moved(src_path: &PathBuf, dest_path: &PathBuf, expected_content: &str) {
    // Check that the source file no longer exists
    assert!(
        !src_path.exists(),
        "Source file still exists at {}",
        src_path.display()
    );

    // Check that the destination file exists and has the correct content
    assert!(
        dest_path.exists(),
        "Destination file does not exist at {}",
        dest_path.display()
    );
    let moved_content = fs::read_to_string(dest_path).unwrap();
    assert_eq!(
        moved_content, expected_content,
        "File content doesn't match after move"
    );
}

#[test]
fn test_move_file_to_directory() {
    // Create a temporary file with content
    let content = "This is a test file for directory move";
    let src_file = create_temp_file(content);
    let src_path = src_file.path().to_path_buf();
    let filename = src_path.file_name().unwrap();

    // Create a temporary directory for the destination
    let dest_dir = tempdir().unwrap();
    let dest_dir_path = dest_dir.path().to_path_buf();

    // Expected destination file path (directory + original filename)
    let expected_dest_path = dest_dir_path.join(filename);

    // Run the mvx command
    let result = run_mvx(&src_path, dest_dir_path.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Failed to move file to directory: {:?}",
        result.err()
    );

    // Verify the file was moved correctly
    verify_file_moved(&src_path, &expected_dest_path, content);
}
