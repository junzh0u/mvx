# mvx - Enhanced File and Directory Move Utility

`mvx` is a command-line utility for moving files and directories with progress bars and enhanced features. It's designed to be a more user-friendly alternative to the standard `mv` command, providing visual feedback during operations and handling cross-device moves gracefully.

## Features

- Move files with visual progress bars
- Merge directories intelligently
- Handle cross-device moves automatically
- Create destination directories as needed
- Quiet mode for scripting

## Usage

```
mvx [OPTIONS] <SOURCE> <DESTINATION>
```

### Options

- `-q, --quiet`: Suppress progress bars and messages
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Examples

Move a file to a new location:
```bash
mvx file.txt /path/to/destination/
```

Move a file and rename it:
```bash
mvx file.txt /path/to/destination/newname.txt
```

Move a directory and merge with an existing directory:
```bash
mvx source_dir/ destination_dir/
```

Move in quiet mode (no progress bars):
```bash
mvx -q large_file.iso /media/backup/
```

## Behavior Documentation

### File Moves

1. **Basic File Move**: When moving a file to a destination that doesn't exist, the behavior depends on whether the path ends with "/".
    - If the destination path ends with "/", `mvx` will create the directory and place the file inside it.
    - If the destination path doesn't end with "/", `mvx` will create all necessary parent directories and move the file to that exact path.

2. **Move to Directory**: When moving a file to an existing directory, the file is placed inside that directory with its original filename.

3. **Overwriting Files**: If the destination file already exists, it will be overwritten by the source file.

4. **Cross-device Moves**: When moving files between different filesystems/devices, `mvx` will automatically copy the file and delete the original, showing progress during the operation.

### Directory Moves

1. **Directory Merge**: When moving a directory to an existing directory, `mvx` will merge the contents, preserving files in the destination that don't conflict with files from the source.

2. **Overwriting Files in Directory Merge**: During a directory merge, if files with the same name exist in both source and destination, the source files will overwrite the destination files.

3. **Preserving Directory Structure**: When merging directories, the entire directory structure from the source is preserved in the destination.

### Error Handling

1. **Nonexistent Source**: If the source file or directory doesn't exist, `mvx` will display an error message and exit without making any changes.

2. **Cannot Create Parent Directory**: If a parent directory cannot be created (e.g., because an intermediate path component is an existing file), `mvx` will display an error message and exit without moving the file.

[MIT License](LICENSE)
