# cpt.run
> A fast, cross-platform Getting Things Done (GTD) workspace with a native desktop shell and a companion TUI.

<!-- TODO: Add public build/test badges when CI pipelines are published. -->

## Table of Contents
- [Highlights](#highlights)
- [Project Status](#project-status)
- [Screenshots](#screenshots)
- [Quick Start](#quick-start)
- [Usage](#usage)
- [Terminal Companion](#terminal-companion)
- [Configuration](#configuration)
- [Development](#development)
- [Release Notes](#release-notes)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)
- [Additional Resources](#additional-resources)

## Highlights
- Native desktop app powered by `iced` for macOS, Windows, and Linux.
- Local first, Privacy-first, AI powered
- Capture : Capture all that needs to be done
- Plan : Plan and organize using best practices
- Track : Track progress
- Run : Execute with focus


## Project Status
- **Desktop:** Phase 1 (core infrastructure) actively in progress.
- **CLI/TUI:** V1 scaffolding in place.
- See the [roadmap](#roadmap) for the current delivery focus.

## Screenshots
> Screenshots and demo recordings are coming soon. Want to help? Open an issue or PR with assets!

## Quick Start
### Prerequisites
- Rust toolchain (Rust 1.75+ recommended).
- `mise` for task automation (optional but simplifies installs).

### Run the Desktop Shell
```bash
cargo run -- desktop           # debug build with faster iteration
cargo run --release -- desktop # optimized build for demos
```

- `mise run install` builds a release binary and copies it to `bin/cpt` for reuse by the desktop launcher scripts.
- Prebuilt binaries live in `bin/`. Symlink it onto your `PATH` if desired: `ln -sf "$PWD/bin/cpt" "$HOME/.local/bin/cpt"`.

### macOS Desktop Bundle
Create a signed `.app` bundle for the desktop shell:
```bash
# first time only: cargo install cargo-bundle
mise run bundle-desktop-macos
open target/release/bundle/osx/   # contains cpt.run Desktop.app
```
The helper regenerates `crates/desktop/icons/cpt.icns`, runs `cargo bundle`, and produces an app bundle ready for codesigning or distribution.

## Usage
### Desktop App
- Navigate GTD lists (All, Inbox, Next, Waiting, Scheduled, Someday, Projects, Done) with native controls.
- Capture new commitments using inline tokens such as `@context`, `+project`, and `due:tomorrow`.
- Review task details, promote or defer items, and mark work complete directly inside the desktop shell.
- Desktop and terminal surfaces share the same SQLite storage, so updates appear instantly everywhere.

## Terminal Companion
> Prefer the keyboard-driven workflow? The terminal UI offers the same GTD semantics with dedicated shortcuts.

### Launch the TUI
```bash
cargo run -- tui
cargo run --release -- tui
# once built with `mise run install`:
# cpt tui
```

### Key Bindings
- `Tab` / `Shift+Tab` switch GTD views (All, Inbox, Next, Waiting, Scheduled, Someday, Projects, Done).
- `j`/`k` or `↓`/`↑` move the selection.
- `a` opens the capture prompt (supports inline tokens like `@context`, `+project`, `due:tomorrow`).
- `Enter` opens the detail panel for the highlighted task (press `Enter`/`Esc` to close).
- `n` promotes the highlighted task into Next actions.
- `s` moves the highlighted task into Someday/Maybe.
- `i` sends the highlighted task back to the Inbox.
- `e` opens `/edit <id>` with the selected task pre-filled.
- `f` opens the filter picker (projects, contexts, tags, and priority).
- `d` marks the selected task as done.
- `r` refreshes the view, `q` exits.

### Filtering
- Press `f` to open the filter picker. `←/→` switch columns, `↑/↓` move within a column.
- `Space` toggles selections, `C` clears all, `Enter` applies filters.
- The header displays active filters. Select the first row of any column or press `C` to clear that facet.
- `/filter clear` from the command palette clears all filters.

## Configuration
- Debug builds (`cargo run`) store SQLite data in `tmp/dev-cpt`.
- Release binaries (`cargo run --release`, installed builds, packaged desktop app) use the platform-specific application directory.
- Override storage location with the `--data-dir` flag or `CPT_DATA_DIR` environment variable.
- The database initializes automatically on first launch.

## Development
Repository structure (top-level `desktop/` directory shown):
- `src/`: Main binary entry point that dispatches to desktop, TUI, command, and MCP flows.
- `crates/core/`: `cpt-core` library with domain models, configuration, parser, storage, and command helpers.
- `crates/tui/`: `cpt-tui` crate powering the terminal interface.
- `crates/desktop/`: `cpt-desktop` iced-based desktop shell library.
- `crates/mcp/`: `cpt-mcp` MCP server for agent integrations.
- `scripts/`: Automation helpers for builds, packaging, and data migrations.

Common dev commands:
```bash
cargo run -- desktop
cargo run -- tui
cargo run --bin cpt-mcp
```

## Release Notes
- Application version comes from `Cargo.toml` and renders in the TUI header as `cpt.run vX.Y.Z`.
- Public release notes will be published alongside tagged releases.

## Roadmap
- Phase 1: finalize core infrastructure for the desktop shell.
- Upcoming focus: fold MCP feedback into the shared core, polish packaging, and document automation entry points.
- Track feature proposals and milestones via GitHub issues once the tracker is public.

## Contributing
- Issues and pull requests are welcome once the tracker is public.
- Please review open roadmap items before proposing UX changes.
- Keep onboarding hints (`/help`, capture prompts) in sync with any string updates.

## License
- Licensed under the GNU General Public License v3.0; see `LICENSE` for full terms.
- Contributions must be GPLv3-compatible and include source when distributing binaries.

## Additional Resources
- Desktop crate: `crates/desktop/`
- MCP crate: `crates/mcp/`
- License details: `LICENSE`
