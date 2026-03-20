## pending changes

### Features

- Recursive directory scanning with file and directory counts (via jwalk)
- Hierarchical tree view with expandable/collapsible folders
- Sort by name, total size, file count, or average file size
- Percentage-of-parent visualization for each directory
- Open files and folders directly in Windows Explorer
- Duplicate file detection with three-phase algorithm:
  - Group files by size
  - Partial hash comparison (first 4 KB)
  - Full hash comparison (BLAKE3)
- Configurable minimum file size threshold for duplicate detection
- Real-time progress bar with speed and ETA estimates during duplicate scanning
- Live candidate display while scanning is in progress
- Emoji indicators in the UI (folders, files, warnings)
- GUI built with eframe / egui: toolbar, tree view, duplicates view

### Fixes

- Remove unused code and fix compilation warnings
