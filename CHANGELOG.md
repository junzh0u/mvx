# Changelog

## v0.2.9 - 2026-04-07

### Features

- Exclude renamed/reflinked files from throughput summary

### Fixes

- Include file path in error output via anyhow context

### Performance

- Skip recursive walk when moving directory to non-existent dest (single rename)
- Rename non-overlapping subdirectories wholesale during merge
- Skip `collect_total_size` and progress bar for same-device moves

### Docs

- Add crates.io install option, fix verbosity flag descriptions

### Chores

- Populate GitHub Release notes from CHANGELOG.md

## v0.2.8 - 2026-03-21

### Features

- Auto-create dest directory for multiple sources (dirs always; files with trailing `/`)
- Dim individual completion messages only in batch mode
- Batch summary with bold/italic progress labels
- Dim completion messages and show average transfer speed
- Batch progress bar showing total size and item count for multi-source operations

### Fixes

- Add `-f` flag to cpx calls in `make_fixtures.sh` to allow re-running
- Hide batch progress bar for single-source operations

### Refactoring

- Remove `SourceKind::Mixed`; `validate_sources` returns `SourceKind` directly
- Replace `is_dir` bool with `SourceKind` enum; consolidate message formatting into `Ctx` methods
- Inline single-caller methods and extract shared helpers from `run_batch`
- Use integer arithmetic in `human_speed` to eliminate clippy cast suppressions

### Chores

- Upgrade indicatif 0.17.11 -> 0.18.4
- Replace Makefile with justfile, add `just fix` command
- Add `deploy-linux` just command for cross-compiling

### Docs

- Replace scattered behavior sections with source/dest behavior matrix in README
- Fix stale CLAUDE.md references

## v0.2.7 - 2026-02-07

### Features

- Force exit on double Ctrl-C

### Fixes

- Let current file finish on first Ctrl+C, abort on second
- Make Ctrl+C responsive during large file transfers
- Preserve empty directories and clean up incrementally during moves
- Use `_exit()` for force Ctrl-C to avoid deadlock on cleanup

### Performance

- Remove double buffering in `buffered_copy`

### Refactoring

- Use recursive DFS for directory merge/copy
- Introduce `Ctx` struct to bundle session-level state

## v0.2.6 - 2026-02-05

### Features

- Add dry-run option (`-n`/`--dry-run`)
- Support `MODE_DRY_RUN` environment variable for dry-run mode
- Overwrite protection (`-f` flag)
- Ctrl+C handling

### Refactoring

- Add `ensure_dest` for destination resolution
- Simplify progress handling and add batch validation

## v0.2.5 - 2025-11-03

### Features

- Total bytes progress bar

### Fixes

- Total bytes progress tracking

### Refactoring

- Redesign messages and progress bars
- Pass progress handler around instead of progress bars

## v0.2.4 - 2025-07-18

### Features

- Sort files while merging directories

## v0.2.3 - 2025-06-10

### Fixes

- Finish message display

## v0.2.2 - 2025-06-09

### Refactoring

- Simplify progress bar handling
- Use `AsRef<[Src]>` for flexible source input

## v0.2.1 - 2025-06-02

### Features

- Accept multiple source arguments

## v0.2.0 - 2025-05-31

### Features

- Add `cpx` (copy) binary
- Fallback for any reflink error

### Refactoring

- Split into `file`, `dir`, and `bin` modules with unit tests
- Use `fs::rename` CrossesDevices error instead of manual device comparison

## v0.1.3 - 2025-05-29

### Features

- Colored error messages
- Different symbols for move and merge
- Logging support

### Fixes

- Skip steady tick when progress bar is hidden

## v0.1.2 - 2025-05-28

### Fixes

- Only remove source directory if empty

## v0.1.1 - 2025-05-28

### Fixes

- Graceful exit handling
- Handle case where dest parent cannot be determined
- Fix `mvx a b` single file rename

## v0.1.0 - 2025-05-27

Initial release: enhanced `mv` command with directory merging, progress bars, and quiet mode.
