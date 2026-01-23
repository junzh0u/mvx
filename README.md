# mvx/cpx - Enhanced File and Directory Move Utility

`mvx`/`cpx` is a command-line utility that extends the standard `mv`/`cp` command with progress bars and enhanced features.

For basic file operations, `mvx`/`cpx` behaves the same as the standard `mv`/`cp` command.

## Key Features / Differences from `mv`/`cp`

### Directory Handling
When moving/copying a directory to an existing directory, `mvx`/`cpx` merges the contents rather than replacing or nesting the directory. Unique files in the destination are preserved. By default, existing files are not overwritten; use `-f` to allow overwriting.

### Path Creation
`mvx`/`cpx` automatically creates any necessary destination directories.

### Progress Visualization
For cross-device file operations, `mvx`/`cpx` displays progress bars, which can be suppressed via `-q` flag.

## Usage

> [!NOTE]
> only `mvx` is demonstrated in this section, `cpx` works exactly the same.

```
mvx [OPTIONS] <SOURCE> <DESTINATION>
```

### Options

- `-f, --force`: Overwrite existing files
- `-q, --quiet`: Suppress progress bars and messages
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Examples

```bash
# Move a file with progress bar
mvx large_file.iso /media/backup/

# Move and merge a directory
mvx source_dir/ destination_dir/

# Move and overwrite existing files
mvx -f source_dir/ destination_dir/

# Move in quiet mode
mvx -q large_file.iso /media/backup/
```

[MIT License](LICENSE)
