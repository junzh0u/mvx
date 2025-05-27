use anyhow::Result;
use clap::Parser;
use mvx::{run, Cli};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::{tempdir, NamedTempFile};

// Helper function to create a temporary file with content
fn create_temp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

// Helper function to create a temporary directory with files
fn create_temp_dir_with_files() -> (tempfile::TempDir, Vec<PathBuf>) {
    let dir = tempdir().unwrap();
    let mut file_paths = Vec::new();

    // Create a few files in the directory
    for i in 1..4 {
        let file_path = dir.path().join(format!("file{}.txt", i));
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "Content of file {}", i).unwrap();
        file_paths.push(file_path);
    }

    // Create a subdirectory with files
    let subdir_path = dir.path().join("subdir");
    fs::create_dir(&subdir_path).unwrap();
    for i in 1..3 {
        let file_path = subdir_path.join(format!("subfile{}.txt", i));
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "Content of subdir file {}", i).unwrap();
        file_paths.push(file_path);
    }

    (dir, file_paths)
}

// Helper function to run mvx command with quiet mode
fn run_mvx(src: &PathBuf, dest: &str) -> Result<()> {
    let cli = Cli::parse_from(["mvx", "-q", src.to_str().unwrap(), dest]);
    run(cli)
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

// Helper function to create a file with content at a specific path
fn create_file_with_content(path: &PathBuf, content: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "{}", content)?;
    Ok(())
}

#[test]
fn test_move_single_file() {
    // Create a temporary file with content
    let content = "This is a test file";
    let src_file = create_temp_file(content);
    let src_path = src_file.path().to_path_buf();

    // Create a temporary directory for the destination
    let dest_dir = tempdir().unwrap();
    let dest_path = dest_dir.path().join("moved_file.txt");

    // Run the mvx command
    let result = run_mvx(&src_path, dest_path.to_str().unwrap());
    assert!(result.is_ok(), "Failed to move file: {:?}", result.err());

    // Verify the file was moved correctly
    verify_file_moved(&src_path, &dest_path, content);
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

// Helper function to create pre-existing files in a directory
fn create_pre_existing_files(dir_path: &PathBuf) -> (Vec<PathBuf>, Vec<String>) {
    let mut files = Vec::new();
    let mut contents = Vec::new();

    // Create regular files
    for i in 1..3 {
        let file_path = dir_path.join(format!("existing_file{}.txt", i));
        let content = format!("Pre-existing content {}", i);
        create_file_with_content(&file_path, &content).unwrap();
        files.push(file_path);
        contents.push(content);
    }

    // Create a subdirectory with a file
    let subdir_path = dir_path.join("existing_subdir");
    fs::create_dir(&subdir_path).unwrap();
    let subfile_path = subdir_path.join("existing_subfile.txt");
    let subfile_content = "Pre-existing subdir file content";
    create_file_with_content(&subfile_path, &subfile_content).unwrap();
    files.push(subfile_path);
    contents.push(subfile_content.to_string());

    (files, contents)
}

// Helper function to verify pre-existing files still exist with unchanged content
fn verify_pre_existing_files(files: &Vec<PathBuf>, contents: &Vec<String>) {
    for (i, file_path) in files.iter().enumerate() {
        assert!(
            file_path.exists(),
            "Pre-existing file {} no longer exists",
            file_path.display()
        );

        // Verify content is unchanged
        let content = fs::read_to_string(file_path).unwrap().trim().to_string();
        assert_eq!(
            content,
            contents[i],
            "Content changed for pre-existing file {}",
            file_path.display()
        );
    }
}

#[test]
fn test_merge_directories() {
    // Create a source directory with files
    let (src_dir, src_files) = create_temp_dir_with_files();
    let src_path = src_dir.path().to_path_buf();

    // Create a destination directory with some pre-existing files
    let dest_dir = tempdir().unwrap();
    let dest_path = dest_dir.path().to_path_buf();

    // Create and track pre-existing files in destination
    let (dest_files, dest_file_contents) = create_pre_existing_files(&dest_path);

    // Store source file contents before moving
    let mut src_file_contents = Vec::new();
    for src_file in &src_files {
        let content = fs::read_to_string(src_file).unwrap();
        src_file_contents.push((
            src_file.strip_prefix(&src_path).unwrap().to_path_buf(),
            content,
        ));
    }

    // Run the mvx command to merge directories
    let result = run_mvx(&src_path, dest_path.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Failed to merge directories: {:?}",
        result.err()
    );

    // Check that the source directory no longer exists
    assert!(!src_path.exists(), "Source directory still exists");

    // Check that all files from source are now in destination with correct content
    for (rel_path, content) in &src_file_contents {
        let dest_file = dest_path.join(rel_path);
        assert!(
            dest_file.exists(),
            "File {} not found in destination",
            dest_file.display()
        );

        // Verify content matches what was in the source
        let moved_content = fs::read_to_string(&dest_file).unwrap();
        assert_eq!(
            &moved_content,
            content,
            "Content doesn't match for moved file {}",
            dest_file.display()
        );
    }

    // Check that the subdirectory was moved
    let moved_subdir_path = dest_path.join("subdir");
    assert!(
        moved_subdir_path.exists() && moved_subdir_path.is_dir(),
        "Subdirectory not found in destination"
    );

    // Check that pre-existing files in destination still exist with unchanged content
    verify_pre_existing_files(&dest_files, &dest_file_contents);

    // Check that pre-existing subdirectory still exists
    let dest_subdir_path = dest_path.join("existing_subdir");
    assert!(
        dest_subdir_path.exists() && dest_subdir_path.is_dir(),
        "Pre-existing subdirectory no longer exists after merge"
    );
}

#[test]
fn test_overwrite_existing_file() {
    // Create a source file with specific content
    let source_content = "This is the source file that should overwrite the destination";
    let src_file = create_temp_file(source_content);
    let src_path = src_file.path().to_path_buf();

    // Create a destination directory
    let dest_dir = tempdir().unwrap();

    // Create a destination file with the same name but different content
    let filename = src_path.file_name().unwrap();
    let dest_path = dest_dir.path().join(filename);
    let dest_content = "This is the original destination file that should be overwritten";
    create_file_with_content(&dest_path, dest_content).unwrap();

    // Verify the destination file exists with its original content
    assert!(
        dest_path.exists(),
        "Destination file was not created properly for test setup"
    );
    let original_dest_content = fs::read_to_string(&dest_path).unwrap();
    assert!(
        original_dest_content.contains(dest_content),
        "Destination file doesn't contain expected original content"
    );

    // Run the mvx command to move the source file to the destination (should overwrite)
    let result = run_mvx(&src_path, dest_path.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Failed to move and overwrite file: {:?}",
        result.err()
    );

    // Verify the destination file now contains the source content
    verify_file_moved(&src_path, &dest_path, source_content);
}

#[test]
fn test_overwrite_file_in_directory_merge() {
    // Create a source directory with files
    let (src_dir, _) = create_temp_dir_with_files();
    let src_path = src_dir.path().to_path_buf();

    // Create a destination directory
    let dest_dir = tempdir().unwrap();
    let dest_path = dest_dir.path().to_path_buf();

    // Create a file in the destination with the same name as one in the source
    let conflict_filename = "file1.txt";
    let dest_conflict_path = dest_path.join(conflict_filename);
    let dest_content = "This is the original destination file that should be overwritten";
    create_file_with_content(&dest_conflict_path, dest_content).unwrap();

    // Get the content of the source file that will overwrite the destination
    let src_conflict_path = src_path.join(conflict_filename);
    let src_content = fs::read_to_string(&src_conflict_path).unwrap();

    // Verify both files exist before the merge
    assert!(
        src_conflict_path.exists(),
        "Source conflict file doesn't exist"
    );
    assert!(
        dest_conflict_path.exists(),
        "Destination conflict file doesn't exist"
    );

    // Run the mvx command to merge directories
    let result = run_mvx(&src_path, dest_path.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Failed to merge directories: {:?}",
        result.err()
    );

    // Verify the destination file was overwritten with source content
    let new_dest_content = fs::read_to_string(&dest_conflict_path).unwrap();
    assert_eq!(
        new_dest_content, src_content,
        "Destination file was not properly overwritten during directory merge"
    );

    // Check that the source directory no longer exists
    assert!(
        !src_path.exists(),
        "Source directory still exists after merge"
    );
}

#[test]
fn test_move_to_nonexistent_directory_path() {
    // Create a temporary file with content
    let content = "This is a test file for moving to a nonexistent directory path";
    let src_file = create_temp_file(content);
    let src_path = src_file.path().to_path_buf();
    let filename = src_path.file_name().unwrap();

    // Create a temporary directory to contain our test directory structure
    let base_dir = tempdir().unwrap();

    // Create a path to a nonexistent directory that ends with "/"
    // We'll use a nested path to ensure multiple directories are created
    let nonexistent_dir_path = base_dir.path().join("new_dir/nested_dir/");
    let dest_path_str = nonexistent_dir_path.to_str().unwrap().to_string() + "/";

    // Verify the directory doesn't exist yet
    assert!(
        !nonexistent_dir_path.exists(),
        "Test directory should not exist before the test"
    );

    // Run the mvx command
    let result = run_mvx(&src_path, &dest_path_str);
    assert!(
        result.is_ok(),
        "Failed to move file to nonexistent directory path: {:?}",
        result.err()
    );

    // Check that the destination directory was created
    assert!(
        nonexistent_dir_path.exists(),
        "Destination directory was not created"
    );
    assert!(
        nonexistent_dir_path.is_dir(),
        "Destination path is not a directory"
    );

    // Check that the file was moved inside the new directory with the original filename
    let dest_file_path = nonexistent_dir_path.join(filename);

    // Verify the file was moved correctly
    verify_file_moved(&src_path, &dest_file_path, content);
}

#[test]
fn test_move_to_nonexistent_file_path() {
    // Create a temporary file with content
    let content = "This is a test file for moving to a nonexistent file path";
    let src_file = create_temp_file(content);
    let src_path = src_file.path().to_path_buf();

    // Create a temporary directory to contain our test directory structure
    let base_dir = tempdir().unwrap();

    // Create a path to a nonexistent file with nonexistent parent directories
    // Note: This path does NOT end with "/"
    let nonexistent_parent_dir = base_dir.path().join("new_dir/nested_dir");
    let dest_file_path = nonexistent_parent_dir.join("new_file.txt");

    // Verify the parent directory doesn't exist yet
    assert!(
        !nonexistent_parent_dir.exists(),
        "Test directory should not exist before the test"
    );

    // Run the mvx command
    let result = run_mvx(&src_path, dest_file_path.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Failed to move file to nonexistent file path: {:?}",
        result.err()
    );

    // Check that the parent directories were created
    assert!(
        nonexistent_parent_dir.exists(),
        "Parent directories were not created"
    );
    assert!(
        nonexistent_parent_dir.is_dir(),
        "Parent path is not a directory"
    );

    // Verify the file was moved correctly
    verify_file_moved(&src_path, &dest_file_path, content);
}

#[test]
fn test_error_cant_create_parent_directory() {
    // Create a temporary file with content
    let content = "This is a test file for testing directory creation failure";
    let src_file = create_temp_file(content);
    let src_path = src_file.path().to_path_buf();

    // Create a temporary directory to contain our test structure
    let base_dir = tempdir().unwrap();

    // Create a file (not a directory) that will be part of the path
    let blocking_file_path = base_dir.path().join("existing_file");
    create_file_with_content(&blocking_file_path, "This file blocks directory creation").unwrap();

    // Verify the blocking file exists and is a file, not a directory
    assert!(
        blocking_file_path.exists(),
        "Blocking file was not created properly"
    );
    assert!(
        blocking_file_path.is_file(),
        "Blocking path should be a file, not a directory"
    );

    // Create a destination path that would require creating a directory through the blocking file
    let impossible_dir_path = blocking_file_path.join("nested_dir");
    let dest_path = impossible_dir_path.join("destination.txt");

    // Run the mvx command - this should fail
    let result = run_mvx(&src_path, dest_path.to_str().unwrap());

    // Verify the operation failed
    assert!(
        result.is_err(),
        "Expected an error when parent directory can't be created, but got success"
    );

    // Check that the error message is appropriate
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("Not a directory")
            || err_msg.contains("directory")
            || err_msg.contains("file exists"),
        "Error message doesn't mention directory creation issue: {}",
        err_msg
    );

    // Verify the source file still exists (wasn't deleted)
    assert!(
        src_path.exists(),
        "Source file was deleted despite the operation failing"
    );

    // Verify the destination file wasn't created
    assert!(
        !dest_path.exists(),
        "Destination file was created despite error"
    );

    // Verify the blocking file wasn't modified
    assert!(
        blocking_file_path.is_file(),
        "Blocking file was converted to a directory"
    );
}

#[test]
fn test_error_nonexistent_source() {
    // Create a path to a non-existent file
    let src_path = PathBuf::from("/tmp/nonexistent_file_for_mvx_test");

    // Make sure it really doesn't exist
    if src_path.exists() {
        fs::remove_file(&src_path).unwrap();
    }

    // Create a temporary directory for the destination
    let dest_dir = tempdir().unwrap();
    let dest_path = dest_dir.path().join("should_not_be_created.txt");

    // Run the mvx command
    let result = run_mvx(&src_path, dest_path.to_str().unwrap());
    assert!(
        result.is_err(),
        "Expected an error for non-existent source, but got success"
    );

    // Check that the error message contains the expected text
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("does not exist"),
        "Error message doesn't mention that source doesn't exist: {}",
        err_msg
    );

    // Check that no destination file was created
    assert!(
        !dest_path.exists(),
        "Destination file was created despite error"
    );
}
