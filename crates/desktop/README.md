# cpt-desktop

Phase 1 desktop bootstrap for the GTD Todo CLI project. The crate exposes reusable helpers that the main `cpt` binary invokes to spin up an `iced` application shell with:

- Native theme detection via `dark-light`
- Periodic database refresh (every 5s) using the existing SQLite store
- Telemetry stubs guarded by the `telemetry` cargo feature (enabled by default)
- Feature flag scaffold for future desktop-only capabilities (e.g., `notifications`)
- Inline editing inside the task list (double-click a title to rename, click the Project/Contexts/Tags/Priority cells to adjust metadata with inline dropdowns)

Launch it through the main CLI (single binary) from the workspace root:

```bash
cargo run -- desktop
# or with the installed binary
cpt desktop
```

Feature flags (determine via workspace-level features before invoking `cpt desktop`):
- `telemetry` – enabled by default.
- `notifications` – compiles optional notification hooks (placeholder for future work).

## macOS bundling

The crate includes `package.metadata.bundle` for [`cargo-bundle`](https://github.com/burtonageo/cargo-bundle). Use the helper script to regenerate the `cpt.icns` asset and emit a distributable `.app` bundle:

```bash
cd ../../..
# first time only: cargo install cargo-bundle
mise run bundle-desktop-macos   # produces target/release/bundle/osx/cpt.run Desktop.app
```

The generated bundle reuses the icon set under `app/icons/` and embeds the product documentation from `docs/user-guide/` as application resources.
