# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build

# Test
cargo test                     # Run all tests
cargo test <test_name>         # Run specific test

# Lint & Format
cargo fmt --all -- --check     # Check formatting
cargo fmt --all                # Auto-format
cargo clippy -- -W clippy::pedantic -D warnings  # Lint with strict settings

# Install
cargo install --path .         # Install mvx and cpx binaries

# All checks (equivalent to CI)
just check                     # Runs fmt, clippy, test
just fix                       # Auto-fix formatting and clippy warnings
```

## Architecture

MVX is a Rust CLI utility providing enhanced `mv`/`cp` commands with directory merging and progress bars.

### Source Structure

```
src/
├── lib.rs          # Core orchestration: run_batch(), logging, Ctrl-C handling
├── file.rs         # Single file operations with rename/reflink fast paths
├── dir.rs          # Directory merging with recursive file collection
└── bin/
    ├── mvx.rs      # Move binary (thin CLI wrapper)
    └── cpx.rs      # Copy binary (thin CLI wrapper)
```

### Key Design Patterns

- **MoveOrCopy enum**: Drives behavior differences throughout the codebase
- **Fast path optimization**: Tries `fs::rename()` (move) or `reflink::reflink()` (copy) first, falls back to buffered I/O with progress bars on cross-device/unsupported errors
- **Directory merging**: `dir::merge_or_copy_recursive()` walks directories via recursive DFS, sorted entries per level, preserves unique destination files
- **Source validation**: `validate_sources()` rejects mixed file/dir sources and returns `SourceKind` (File or Dir)
- **Progress tracking**: `indicatif::MultiProgress` passed through call stack; hidden draw target in quiet mode
- **Ctrl-C handling**: `AtomicBool` checked between operations; exit code 130

### Module Responsibilities

- **lib.rs**: `run_batch()` validates inputs (returning `SourceKind`), dispatches to file/dir modules, handles Ctrl-C between sources
- **file.rs**: `move_or_copy()` handles destination resolution via `ensure_dest()`, creates intermediate directories, manages fast-path fallback
- **dir.rs**: `merge_or_copy()` walks directories via recursive DFS, tracks cumulative progress across files, cleans up empty source directories after move

## Testing Notes

- Tests use `tempfile::tempdir()` for isolation
- Tests marked `#[serial]` in file.rs change the working directory and must run serially
- Run `cargo test -- --test-threads=1` if seeing flaky failures from parallel test execution
