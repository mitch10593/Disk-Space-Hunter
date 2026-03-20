# Disk Space Hunter

A desktop application for analyzing disk usage and detecting duplicate files, built with Rust and [egui](https://github.com/emillegp/egui).

## Features

### Directory Tree Analysis
- Recursive directory scanning with file/directory counts
- Hierarchical tree view with expandable/collapsible folders
- Sort by name, total size, file count, or average file size
- Percentage-of-parent visualization for each directory
- Open files and folders directly in Windows Explorer

### Duplicate File Detection
- Three-phase detection algorithm for accuracy and performance:
  1. Group files by size
  2. Partial hash comparison (first 4 KB)
  3. Full hash comparison using [BLAKE3](https://github.com/BLAKE3-team/BLAKE3)
- Configurable minimum file size threshold
- Real-time progress with speed and ETA estimates
- Wasted space calculation

## Screenshot

<!-- TODO: add screenshot -->

## Building

```bash
cargo build --release
```

The compiled binary will be in `target/release/`.

## Running

```bash
cargo run --release
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `eframe` / `egui_extras` | GUI framework |
| `rfd` | Native file dialogs |
| `jwalk` | Parallel directory walking |
| `blake3` | File hashing |
| `crossbeam-channel` | Thread-safe message passing |

## License

Project licence: [MIT](LICENSE.md)
