# mvx / cpx

Enhanced `mv` and `cp` commands with directory merging and progress bars.

## Installation

```bash
cargo install --path .
```

This installs both `mvx` (move) and `cpx` (copy) binaries.

## Usage

```
mvx [OPTIONS] <SOURCES>... <DEST>
cpx [OPTIONS] <SOURCES>... <DEST>
```

### Options

| Option | Description |
|--------|-------------|
| `-f, --force` | Overwrite existing files |
| `-n, --dry-run` | Show what would be done without actually doing it |
| `-q, --quiet` | Suppress progress bars and info messages |
| `-v, --verbose` | Show detailed output |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Features

### Directory Merging

Unlike standard `mv`/`cp`, when the destination is an existing directory, `mvx`/`cpx` merges contents instead of nesting:

```bash
# Standard mv: creates dest/source_dir/
mv source_dir/ dest/

# mvx: merges source_dir/* into dest/*
mvx source_dir/ dest/
```

Files unique to the destination are preserved. Overlapping files require `-f` to overwrite.

### Safe by Default

Existing files are never overwritten unless `-f` is specified:

```bash
# Fails if dest/file.txt exists
mvx file.txt dest/

# Overwrites dest/file.txt if it exists
mvx -f file.txt dest/
```

### Automatic Directory Creation

Destination directories are created automatically:

```bash
# Creates /path/to/new/dest/ if it doesn't exist
mvx file.txt /path/to/new/dest/
```

### Progress Bars

Cross-device operations display progress bars with transfer speed and ETA:

```
   1.2 GiB [========>-------------------------------]  312 MiB (  45.2 MiB/s, ETA: 00:00:21 ) file.iso -> /mnt/backup/file.iso
```

Use `-q` to suppress progress output.

### Fast Path Optimization

Same-device moves use `rename` (instant). Same-filesystem copies use `reflink` (copy-on-write clone on APFS/Btrfs). The buffered copy fallback with progress bars only kicks in when these fast paths aren't available.

### Ctrl+C Handling

Press Ctrl+C once to finish the current file and stop. Press again to force exit immediately.

## Examples

```bash
# Move a single file
mvx file.txt /backup/

# Copy multiple files to a directory
cpx file1.txt file2.txt /dest/

# Merge directories (preserve existing, fail on conflicts)
mvx source_dir/ dest_dir/

# Merge directories (overwrite conflicts)
mvx -f source_dir/ dest_dir/

# Copy with progress bar suppressed
cpx -q large_file.iso /mnt/usb/
```

## Differences from mv/cp

| Behavior | mv/cp | mvx/cpx |
|----------|-------|---------|
| Directory to existing directory | Nests source inside dest | Merges contents |
| Destination doesn't exist | Fails (for directories) | Creates automatically |
| File exists at destination | Overwrites silently | Fails (use `-f` to overwrite) |
| Cross-device operations | No progress indication | Shows progress bar |
| Same-device moves | `rename` | `rename` (same) |
| Same-filesystem copies | Full copy | `reflink` (instant clone) |
| Ctrl+C | Stops immediately | Finishes current file, then stops |

## License

[MIT License](LICENSE)
