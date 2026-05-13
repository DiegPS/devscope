# AGENTS.md

## Build & Test
- `cargo check` — fast compile check (no warnings expected)
- `cargo test` — runs all 37 unit tests (0.37s after compile)
- `cargo build` — release/debug build
- `cargo clippy` — linting (no custom clippy.toml)

No CI, no pre-commit hooks, no rustfmt.toml.

## Architecture
Entrypoint: `src/main.rs` — if no subcommand, loads config + launches TUI.
Key modules:
- `scanner.rs` — walks root dirs (up to `max_depth`, default 4), detects projects via 28 marker files, uses `rayon` for parallel analysis
- `detect.rs` — tech stack detection from config files (Rust, Node, Python, Go, Flutter, Docker, .NET, Java, etc.)
- `config.rs` — TOML config at platform config dir (e.g. `%APPDATA%/devscope/config.toml` on Windows, `~/.config/devscope/config.toml` on Linux)
- `project.rs` — data models: `Project`, `GitInfo`, `ProjectStatus`, `ProjectWarning`, `HealthLevel`
- `health.rs` — health score 0-100, deductions for missing README, dirty git, env files, etc.
- `app.rs` — TUI state: filters, sorts, search, view modes
- `ui/` — ratatui rendering: table.rs, details.rs, footer.rs, theme.rs, layout.rs
- `tui.rs` — terminal setup + main event loop
- `input.rs` — key event dispatch for all modes
- `scoring.rs` — frecency-based search ranking
- `discover.rs` — auto-discovery of project roots from common dirs (~/dev, ~/projects, etc.)

## TUI Keybindings
| Key | Action |
|-----|--------|
| `q` / `Q` | Quit |
| `Esc` | Cancel / back to Normal mode |
| `/` | Search mode |
| `f` | Cycle filter |
| `s` | Cycle sort |
| `r` | Reload scan |
| `n` | Add/edit note |
| `m` | Change status |
| `o` | Open project (prints path) |
| `Enter` | Record visit |
| `D` | Toggle compact/detailed view |
| `?` | Help overlay |
| `↑↓` / `j k` | Navigate list |

## Gotchas
- `git2` (libgit2) requires CMake + a C compiler on most platforms
- Scanner skips 50 dir names (node_modules, target, .git, dist, etc.) — see `SKIP_DIRS` in `scanner.rs`
- Project detection requires explicit marker files (e.g. `.git`, `package.json`, `Cargo.toml`); dirs without markers are not treated as projects
- Config auto-saves on first run if roots are empty and auto-discovery finds anything
- `open` command is MVP — just prints the path, doesn't launch an editor/file manager
- `cargo test` has zero prerequisites; all tests are pure unit tests using `tempfile`
