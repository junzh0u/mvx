# mvx - Enhanced File and Directory Move Utility

`mvx` is a command-line utility that extends the standard `mv` command with progress bars and enhanced features.

## Features

- **Progress Bars**: Visual feedback during file operations
- **Directory Merging**: Intelligently merges directories instead of replacing them
- **Cross-device Moves**: Handles moves between filesystems with progress indication
- **Auto-create Directories**: Creates destination directories as needed

For basic file operations, `mvx` behaves the same as the standard `mv` command.

## Usage

```
mvx [OPTIONS] <SOURCE> <DESTINATION>
```

### Options

- `-q, --quiet`: Suppress progress bars and messages
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Examples

```bash
# Move a file with progress bar
mvx large_file.iso /media/backup/

# Move and merge a directory
mvx source_dir/ destination_dir/

# Move in quiet mode
mvx -q large_file.iso /media/backup/
```

## Key Differences from `mv`

### Directory Handling
When moving a directory to an existing directory, `mvx` merges the contents rather than replacing or nesting the directory. Files with the same name are overwritten, but unique files in the destination are preserved.

### Progress Visualization
For large files or cross-device moves, `mvx` displays progress bars showing:
- Bytes transferred
- Transfer speed
- Estimated time remaining

### Path Creation
`mvx` automatically creates any necessary destination directories:
- If path ends with "/", creates the directory and places the file inside
- If path doesn't end with "/", creates parent directories and moves to exact path

[MIT License](LICENSE)
