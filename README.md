# Knot GTK4

GTK4 frontend for the Knot knowledge base application.

## Architecture

This project uses the same daemon-backed architecture as Knot:

```
┌─────────────┐      Unix Socket/JSON-RPC      ┌─────────────┐
│  knot-gtk   │  ◄──────────────────────────►  │   knotd     │
│  (GTK4 UI)  │                                │  (daemon)   │
└─────────────┘                                └──────┬──────┘
                                                      │
                                               ┌──────┴──────┐
                                               │ Vault/Notes │
                                               │  (SQLite)   │
                                               └─────────────┘
```

- **knot-gtk**: GTK4/libadwaita UI application
- **knotd**: daemon process that handles vault operations, search, graph, and settings

## Prerequisites

1. Rust toolchain installed
2. `knotd` available on the machine you are developing against

## Building

```bash
# Build the GTK4 app
cargo build --release

# Or run directly
cargo run
```

## Running

1. Start a compatible `knotd` daemon:
```bash
# Example: listen on the default XDG socket path
mkdir -p "$XDG_RUNTIME_DIR/knot"
knotd --vault /path/to/vault --listen-unix "$XDG_RUNTIME_DIR/knot/knotd.sock"
```

2. Run the GTK4 UI:
```bash
cd /path/to/knot-gtk
cargo run
```

### CLI Arguments

```bash
# Use default socket path from XDG_RUNTIME_DIR
cargo run

# Specify custom socket path
cargo run -- --socket /path/to/custom.sock
# or
cargo run -- -s /path/to/custom.sock

# Use environment variable
KNOTD_SOCKET_PATH=/tmp/knotd.sock cargo run
```

### Default Socket Path

The default socket path is determined in this priority:
1. `--socket` CLI argument
2. `KNOTD_SOCKET_PATH` environment variable  
3. `$XDG_RUNTIME_DIR/knot/knotd.sock`

If `XDG_RUNTIME_DIR` is unavailable, pass `--socket` or set `KNOTD_SOCKET_PATH` explicitly. The app does not guess a `/run/user/<uid>` fallback.

## Data Integration

The GTK4 app integrates with knotd via JSON-RPC to provide:

### Vault Operations
- `get_vault_info()` - Get current vault information
- `open_vault(path)` - Open a vault
- `create_vault(path)` - Create and open a new vault
- `is_vault_open()` - Check vault status

### Note Operations
- `get_note(path)` - Load note content
- `save_note(path, content)` - Save note changes
- `create_note(path, content)` - Create new note
- `delete_note(path)` - Delete a note
- `rename_note(old_path, new_path)` - Rename/move note

### Explorer
- `get_explorer_tree()` - Get folder/note tree structure
- `set_folder_expanded(path, expanded)` - Persist folder state
- `create_directory(path)` - Create new folder
- `rename_directory(old_path, new_path)` - Rename folder
- `remove_directory(path, recursive)` - Delete folder

### Search
- `search_notes(query, limit)` - Full-text search

### Graph
- `get_graph_layout(width, height)` - Get graph layout for visualization
- `graph_neighbors(path, depth)` - Get linked notes

### Settings
- `get_vault_settings()` - Read vault settings
- `update_vault_settings(patch)` - Update settings

## Project Structure

```
knot-gtk/
├── Cargo.toml          # Dependencies
├── src/
│   ├── main.rs         # Application entry point
│   ├── client/
│   │   └── mod.rs      # knotd JSON-RPC client
│   └── ui/
│       ├── mod.rs      # UI module exports
│       ├── window.rs   # Main window
│       ├── sidebar.rs  # Notes list sidebar
│       └── editor.rs   # Note editor
└── README.md
```

## Features

- 🪟 **Modern GTK4/libadwaita UI** - Native GNOME look and feel
- 🛤️ **ToolRail** - Switch between Notes, Search, and Graph modes
- 📑 **ContextPanel** - Shows notes list, search results, or graph controls
- 🔍 **InspectorRail** - Details and settings panel
- 📝 **Note Editor** - Full-featured markdown editing
- 🔌 **Daemon Integration** - Full JSON-RPC client for knotd
- 📚 **Note Explorer** - Browse folders and notes from vault
- 🔎 **Live Search** - Real-time search with suggestions
- ⚡ **Lightweight** - Minimal resource usage

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New Note |
| `Ctrl+W` | Close |
| `F9` | Toggle Sidebar |
| `Ctrl+Q` | Quit |

## Verification Gates

This repository includes local git gate scripts in `scripts/` and repo-local hooks in `.githooks/`.

### Gate Commands

- `just pre-commit-gate`: run the staged-work gate
- `just full-gate`: run the strongest local gate
- `just verify`: alias for the pre-commit gate
- `just install-hooks`: configure `core.hooksPath=.githooks`
- `bash scripts/install-hooks.sh`: install hooks without `just`

### Pre-commit Policy

The pre-commit gate checks:

- staged documentation evidence
- `CHANGELOG.md` presence and Common Changelog structure
- staged `CHANGELOG.md` updates for staged implementation, tooling, hook, docs, and policy changes
- `cargo fmt --check`
- `cargo check`
- Rust tests
- property-test detection and reporting
- fuzz smoke run if a runnable harness exists

If staged changes include code or build files under `src/`, `scripts/`, `.githooks/`, `Cargo.toml`, `Cargo.lock`, `build.rs`, or `justfile`, then the commit must also stage documentation updates under `docs/` or `README.md`.

If staged changes include `src/`, `scripts/`, `.githooks/`, `Cargo.toml`, `Cargo.lock`, `build.rs`, `justfile`, `README.md`, `AGENTS.md`, or `docs/`, then the commit must also stage an update to `CHANGELOG.md` in Common Changelog format.

Docs-only commits are allowed.

## Changelog Policy

This repository maintains a root `CHANGELOG.md` using the Common Changelog standard.

- required heading: `# Changelog`
- allowed group headings: `Added`, `Changed`, `Fixed`, `Removed`
- update `CHANGELOG.md` for any task-completion change in code, tooling, hooks, docs policy, or workflow
- stage the changelog update in the same commit as the implementation or policy change

### Full Gate

The full gate re-runs the pre-commit gate and then runs the strongest available local checks again:

- full Rust test suite
- property-test detection
- full fuzz run if a runnable harness exists

If a fuzz harness is detected but no runnable backend or target can be determined, the gate fails instead of claiming success.

## License

MIT OR Apache-2.0 (same as Knot)
