# devscope

`devscope` is a fast local project scanner plus terminal dashboard for developers.

It is built for the case where you have many folders, many repos, mixed stacks, and you want one place to answer:

- what projects exist under my roots
- what stack each one uses
- which ones are active, stale, dirty, or missing basics
- what branch, remote, upstream, and activity they have
- what commands, artifacts, ports, notes, and status belong to each project

## What it can do

`devscope` can:

- scan one or more root folders and discover projects automatically
- detect stacks, frameworks, package managers, scripts, commands, artifacts, Git state, activity, notes, health, and ports
- rank, filter, search, and inspect projects in a TUI
- open projects through configurable actions
- print project data in human-readable text or JSON from the CLI
- persist notes, manual status, and usage scores in config

## What it builds per project

Each detected project becomes a `Project` model with data such as:

- `name` and `path`
- `stack` labels like `Rust`, `Node`, `Flutter/Dart`, `Python`, `Docker`, `.NET`, `Java`, `C/C++`, `Ruby`, `Swift`
- package manager
- detected scripts
- Git branch, upstream, ahead/behind, remote, last commit, dirty state
- activity timestamps plus relative activity text
- health score with visible positives and warnings
- suggested commands
- build artifacts
- listening TCP ports that match project processes
- user note
- manual or inferred status

## Quick start

Run the TUI:

```bash
devscope
```

Add a root and scan it:

```bash
devscope add-root C:\Users\me\projects
devscope scan
```

List everything as JSON:

```bash
devscope list --json
```

Annotate a project:

```bash
devscope note devscope "Needs perf review"
devscope status devscope active
```

If no roots are configured, `devscope` can auto-discover likely roots on first run.

## CLI reference

Run without a subcommand to launch the TUI:

```bash
devscope
```

Available commands:

```text
devscope scan
devscope list [--json]
devscope add-root <path>
devscope remove-root <path>
devscope roots
devscope note <project> <text>
devscope status <project> <active|paused|stale|archived>
devscope config [--edit]
devscope open <project>
devscope discover [--apply]
```

### Command behavior

- `scan`
  Scans configured roots and prints a compact summary.
- `list`
  Prints a simple list with name, stack, status, activity, and path.
- `list --json`
  Prints the full serialized internal `Project` array.
- `add-root <path>`
  Adds a directory to the roots list in config.
- `remove-root <path>`
  Removes a configured root.
- `roots`
  Shows configured roots.
- `note <project> <text>`
  Saves a note for a project.
- `status <project> <new_status>`
  Persists a manual status.
- `config`
  Prints the path to `config.toml`.
- `config --edit`
  Also prints the config path and tells you to edit it manually.
- `open <project>`
  Resolves a project and prints its path.
- `discover`
  Shows likely root folders found automatically.
- `discover --apply`
  Adds discovered roots into config.

### Project identifier matching

The `project` argument accepted by `note`, `status`, and `open` can be:

- an exact project name
- a partial project name
- a path

Resolution tries path first when the input looks like a path, then exact name, then partial match.

### CLI output

`scan` prints a compact row-oriented summary with:

- project name
- stack
- relative activity
- status
- Git summary

`list --json` exposes the full project model, including:

- `id`, `name`, `path`
- `stack`, `manager`, `scripts`
- `git`
- `activity`
- `status`, `note`
- `warnings`
- `commands`
- `health`
- `artifacts`
- `ports`

`list --json` is useful for scripting, but it is the serialized internal model, not a versioned stable public API.

## TUI

The TUI is where `devscope` is strongest.

It supports:

- search
- filters
- sorts
- compact and detailed views
- notes
- manual status changes
- open action menu
- config action menu
- background Git hydration
- background port detection
- help overlay

### Main keys

- `q` / `Q`: quit
- `Esc`: cancel or clear transient state
- `Up`, `Down`, `j`, `k`: move selection
- `PageUp`, `PageDown`: jump by 10
- `Home`, `End`: first or last project
- `/`: search mode
- `f`: cycle filter
- `s`: cycle sort
- `r`: reload scan
- `n`: edit note
- `m`: change status
- `o`: open action menu
- `,`: config action menu
- `D`: toggle compact / detailed
- `Enter`: record visit
- `?`: open help

### TUI modes

- Search mode
  Type to filter, `Backspace` deletes, `Enter` accepts, `Esc` cancels.
- Note mode
  Type to edit, `Backspace` deletes, `Enter` saves.
- Status mode
  `Up` / `Down` choose a status, `Enter` confirms.
- Open menu
  Press the action key shown in the footer.
- Config menu
  Press the action key shown in the footer.

### Filters

Current filters:

- `all`
- `active`
- `dirty`
- `stale`
- `paused`
- `archived`
- `flutter`
- `rust`
- `node`
- `python`
- `go`
- `docker`
- `windows`
- `with-notes`

### Sorts

Current sorts:

- `activity`
- `name`
- `stack`
- `status`
- `dirty`
- `path`
- `score`

### View modes

- `compact`
  Single-pane list view that uses the full width for the table.
- `detailed`
  Split view with the project table plus a details panel.

In narrow terminals, detailed view switches to a vertical split automatically.

## Project discovery and scanning

`devscope` walks configured roots and treats a directory as a project when it finds a recognized marker file.

Markers include:

- `.git`
- `package.json`
- `pnpm-lock.yaml`
- `yarn.lock`
- `package-lock.json`
- `Cargo.toml`
- `go.mod`
- `pyproject.toml`
- `requirements.txt`
- `Pipfile`
- `poetry.lock`
- `pubspec.yaml`
- `composer.json`
- `pom.xml`
- `build.gradle`
- `build.gradle.kts`
- `settings.gradle`
- `Dockerfile`
- `docker-compose.yml`
- `docker-compose.yaml`
- `CMakeLists.txt`
- `Makefile`
- `Gemfile`
- `Package.swift`
- `deno.json`
- `deno.jsonc`
- `setup.py`
- `setup.cfg`
- `.sln`
- `.csproj`

The scanner skips heavy or noisy directories such as:

- `node_modules`
- `.git`
- `target`
- `dist`
- `build`
- `out`
- `.next`
- `.nuxt`
- `.svelte-kit`
- `.dart_tool`
- `.idea`
- `.vscode`
- `vendor`
- `__pycache__`
- `.venv`
- `venv`
- `env`
- `.gradle`
- `.mvn`
- `coverage`
- `.cache`
- `Pods`
- `bin`
- `obj`

## Stack and framework detection

`devscope` can infer:

- Flutter/Dart and platform folders for Windows, Android, iOS, Web, Linux, macOS
- Node and frameworks/tools like React, Vue, Svelte, Next.js, Nuxt, Vite, Tailwind, Electron, Tauri, Express, Fastify, Angular, TypeScript
- package manager labels such as `pnpm`, `yarn`, `npm`, `Bun`, `Deno`
- Rust and crates like Ratatui, Tauri, Axum, Actix, Bevy, Tokio, Serde
- Go
- Python and frameworks/libraries like FastAPI, Django, Flask, PyTorch, TensorFlow, NumPy, Pandas
- Docker and Compose
- `.NET` / `C#`
- Java with Maven or Gradle
- Kotlin
- `C/C++` with CMake
- Ruby and Rails
- Swift
- DB / migration-oriented projects

## Suggested commands

`devscope` does not run project commands automatically, but it can suggest likely commands based on stack and files.

Examples:

- Node: `npm run dev`, `pnpm build`, `yarn test`, install commands
- Flutter: `flutter pub get`, `flutter run -d windows`, `flutter build web`, `flutter build apk`
- Rust: `cargo run`, `cargo build`, `cargo test`, `cargo build --release`
- Go: `go run .`, `go build`, `go test ./...`
- Python: `pytest`, `pip install -r requirements.txt`, framework-specific serve commands
- Docker: `docker compose up`, `docker compose up --build`, `docker compose down`
- `.NET`: `dotnet run`, `dotnet build`, `dotnet test`
- Java: Maven or Gradle run, test, and build commands

## Git support

For Git repos, `devscope` reads:

- current branch
- last commit hash, message, date, and timestamp
- remote URL, sanitized when needed
- upstream branch
- ahead/behind counts
- parsed remote host/repo
- dirty state
- modified count
- untracked count

The TUI uses a fast first pass and hydrates expensive working tree status in the background.

CLI `scan` and `list` hydrate Git status before printing so their output stays complete.

## Activity

`devscope` computes activity from project file timestamps plus Git commit timestamps when available.

That activity feeds:

- relative labels such as `3m`, `2h`, `5d`, `2mo`, `1y`
- inferred project status when no manual status exists
- sorting by activity
- part of the health heuristics

Automatic status is roughly:

- recent activity => `active`
- older activity => `stale`
- no useful signal => `unknown`

Manual status overrides can still be saved through config or the TUI.

## Health score

Health is a derived score from `0` to `100`.

It rewards:

- README present
- `.gitignore` present
- detected commands
- clean working tree
- remote configured
- mainline branch

It penalizes things such as:

- missing README
- missing `.gitignore`
- `.env` files
- no commands
- no remote
- no upstream
- dirty working tree
- many uncommitted files
- non-mainline branch
- ahead/behind or diverged branch
- stale branch or low activity
- mixed Node lockfiles
- no Git repository

Levels:

- `80..=100` => `good`
- `50..=79` => `warn`
- `0..=49` => `bad`

Health warnings shown in the UI are intended to explain why the score changed.

## Artifacts

Artifact detection currently includes:

- Flutter Windows executables and release folders
- Flutter Android APKs
- Flutter web build output
- Flutter Linux and macOS bundles when present
- Rust debug and release binaries
- Tauri release and bundle outputs
- common Node output folders like `dist`, `build`, and `out`

## Ports

`devscope` can detect listening TCP ports that belong to running project processes.

How it works:

- enumerate active TCP listeners
- inspect the owning process command line
- match that command line against known project paths
- attach matching ports to the corresponding project

Port detection runs asynchronously after reload, so ports may appear shortly after the first render.

## Configuration

Config lives at:

- Windows: `%APPDATA%/devscope/config.toml`
- Linux/macOS: the platform config directory from `directories::ProjectDirs`

Important top-level fields:

```toml
roots = ["C:\\Users\\me\\projects"]
max_depth = 4
respect_gitignore = true
scan_hidden = false
follow_symlinks = false

[ui]
theme = "default"
show_icons = true
right_panel = true

[open]
default = "cursor"

[project_status]
"C:\\Users\\me\\projects\\devscope" = "active"

[notes]
"C:\\Users\\me\\projects\\devscope" = "Needs perf review"
```

### Open actions

Open actions are configurable and shown from the TUI open menu.

Example:

```toml
[[open.actions]]
key = "c"
name = "cursor"
command = "cursor"
args = ["{path}"]
current_dir = false
terminal_mode = false
kind = "command"

[[open.actions]]
key = "g"
name = "lazygit"
command = "lazygit"
args = []
current_dir = true
terminal_mode = true
kind = "command"

[[open.actions]]
key = "f"
name = "folder"
kind = "file_manager"
```

Supported `open.actions` fields:

- `key`
  Single-character trigger used in the menu.
- `name`
  Label shown in the UI.
- `command`
  Executable to run. Optional for special `kind`s.
- `args`
  Arguments. Supports `{path}` and `{name}` placeholders.
- `current_dir`
  Run the command inside the project directory.
- `terminal_mode`
  Suspend the TUI and run interactively in the terminal.
- `env`
  Extra environment variables.
- `kind`
  One of `command`, `file_manager`, `build_output`, or `executable`.

Default action sets include common editors and tools such as:

- `cursor`
- `vscode`
- `nvim`
- `helix`
- `lazygit`
- `yazi`
- `terminal`
- `folder`
- `build output`
- `executable`

Notes, statuses, and usage scores are persisted in config.

## Discovery

`devscope discover` looks for likely root folders under common developer locations such as:

- `~/dev`
- `~/projects`
- `~/source`
- `~/workspace`
- `~/code`
- `~/repos`
- `Documents`
- `Desktop`
- common OneDrive-backed variants

Each candidate gets a confidence level like `HIGH`, `MEDIUM`, or `LOW`.

Use:

```bash
devscope discover --apply
```

to add discovered roots into config.

## Build and test

```bash
cargo check
cargo test
cargo clippy -- -D warnings
```

For local CLI help:

```bash
cargo run -- --help
```

## Requirements

- Rust toolchain
- `git2` / `libgit2` build prerequisites
- on many systems, a C compiler and CMake

## Current limitations

- CLI `open` only resolves and prints the project path; richer launching behavior lives in the TUI open menu
- TUI layout is responsive, but final validation is still mostly manual because there are no snapshot render tests yet
- some config fields exist before every scanner path fully honors them consistently
- JSON output reflects the serialized internal project model, not a versioned contract
