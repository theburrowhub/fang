# Fang

A modern, fast terminal file explorer written in Rust.

## Features

- **3-Panel Layout**: Sidebar | File List | Preview — responsive to terminal size
- **Syntax Highlighting**: Powered by syntect with base16-ocean.dark theme
- **Fuzzy Search**: Real-time fuzzy filtering with SkimMatcherV2 (`/` to activate)
- **Makefile Integration**: Parse and run make targets directly (`m` key)
- **Binary Preview**: Shows file type and size for binary files
- **Async**: Non-blocking UI via tokio + mpsc channels
- **Panic-safe**: Custom hook restores terminal before printing panic info

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Navigate down |
| `k` / `↑` | Navigate up |
| `h` / `←` | Go to parent directory |
| `l` / `→` / Enter | Enter directory or preview file |
| `/` | Start fuzzy search |
| `Esc` | Close search / modal |
| `m` | Open Makefile targets modal |
| `Enter` (in Make modal) | Run selected target |
| `s` | Toggle sidebar |
| `p` | Toggle preview |
| `Tab` | Cycle between panels |
| `q` / `Ctrl+C` | Quit |

## Installation

### From source

```bash
git clone https://github.com/theburrowhub/fang
cd fang
cargo build --release
./target/release/fang
```

## Usage

```bash
fang [directory]   # Open in specified directory (default: current dir)
```

## Architecture

```
src/
├── main.rs              # Event loop (tokio::select!, terminal setup)
├── app/
│   ├── state.rs         # AppState — single source of truth
│   ├── events.rs        # Event enum (Key, PreviewReady, MakeOutput, ...)
│   └── actions.rs       # Action enum + key→action mapping per mode
├── fs/
│   ├── browser.rs       # Directory loading, sorting
│   └── metadata.rs      # FileEntry, FileType, format_size
├── preview/
│   ├── mod.rs           # Preview dispatcher
│   ├── text.rs          # Syntax highlighting via syntect
│   ├── binary.rs        # Binary detection + mime hints
│   └── makefile.rs      # Makefile-specific preview with syntax coloring
├── search/
│   └── fuzzy.rs         # SkimMatcherV2 fuzzy filtering
├── commands/
│   └── make.rs          # Makefile parser + async make execution
└── ui/
    ├── layout.rs        # Responsive 3-panel layout
    ├── utils.rs         # Shared UI utilities
    └── components/
        ├── sidebar.rs   # Directory tree panel
        ├── file_list.rs # File listing panel
        ├── preview.rs   # Preview panel
        ├── footer.rs    # Dynamic keybindings footer
        └── make_modal.rs # Make target modal
```

## Technical Highlights

- **No Arc/Mutex**: State is owned exclusively by the event loop thread
- **OnceLock for syntect**: SyntaxSet initialized once, reused thereafter
- **Async preview loading**: Background tokio tasks send results via mpsc channels
- **Async directory loading**: Non-blocking directory reads via tokio spawn
- **Dirs-first sorting**: Directories always appear before files, both alphabetically sorted
