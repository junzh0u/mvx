# mvx/cpx - Enhanced File and Directory Move Utility

`mvx`/`cpx` is a command-line utility that extends the standard `mv`/`cp` command with progress bars and enhanced features.

## Features

- **Progress Bars**: Visual feedback during file operations
- **Directory Merging**: Intelligently merges directories instead of replacing them
- **Cross-device Moves**: Handles moves/copies between filesystems with progress indication
- **Auto-create Directories**: Creates destination directories as needed

For basic file operations, `mvx`/`cpx` behaves the same as the standard `mv`/`cp` command.

## Key Differences from `mv`/`cp`

### Directory Handling
When moving/copying a directory to an existing directory, `mvx`/`cpx` merges the contents rather than replacing or nesting the directory. Files with the same name are overwritten, but unique files in the destination are preserved.

### Progress Visualization
For cross-device file operations, `mvx`/`cpx` displays progress bars showing:
- Bytes transferred
- Transfer speed
- Estimated time remaining

### Path Creation
`mvx`/`cpx` automatically creates any necessary destination directories.

## Usage

> [!NOTE]
> only `mvx` is demonstrated in this section, `cpx` works exactly the same.

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

[MIT License](LICENSE)
