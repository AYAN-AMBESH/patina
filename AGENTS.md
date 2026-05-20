# Patina

A Rust TUI for NetworkManager — `nm-connection-editor` power over SSH.

## Stack

- Rust 2024 edition, `stable` toolchain (`rust-toolchain.toml`)
- TUI: `ratatui` + `crossterm`
- Async/threads: std threads + mpsc messages (no tokio yet)
- CLI: `clap` derive; logging: `env_logger` → file

## Architecture

```
main.rs
└── Application
    ├── Crossterm event thread  ──► mpsc ──►
    ├── Timer tick thread (fps)   ──► mpsc ──► run loop
    └── Pane (enum_dispatch)
        └── HomeData
            ├── ConnectedList
            ├── AvailableList
            ├── Authenticate (overlay)
            └── Info (overlay)
```

### Component System

`src/application/component.rs` defines:

- `Component` trait: `update(&mut self, ctx, Message) -> Bubble`, `draw(&mut self, ctx, frame)`
- `Bubble::Yes(Message)` bubbles an event up to the parent; `Bubble::No` consumes it.
- `Message` enum: crossterm events, ticks, and domain messages (`LoadConnected`, `Authenticate`, `OpenInfo`, etc.)
- `Context` (read-only config) vs `RichContext` (includes `mpsc::Sender<Message>`)

### Application Loop (`src/application/mod.rs`)

- `run()` polls mpsc messages, draws via `ratatui::run`, breaks on `q`.
- `update()` and `draw()` dispatch to the active `Pane`.
- Background work spawns std threads and sends results back as `Message`s.

### Home Pane (`src/application/home.rs`)

- Two scrollable lists: **Connections** (top) and **Access Points** (bottom).
- `Selected` enum tracks which list/pane is focused; `j`/`k` navigate, `Tab` switches panes.
- `sync_scroll_states()` couples selection to `ScrollState` after every movement.
- Overlays (`Authenticate`, `Info`) intercept events first; unhandled events bubble to the pane.
- `available_max_items` is a `Cell<usize>` updated at render time so key handling matches the drawn viewport.

### Rendering Conventions

- Theme: `PATINA` in `theme.rs` — warm dark palette (`bg: #16100C`, `accent: #E99355`, `live: #6FC4A0`).
- Heavy use of ratatui macros: `span!`, `line!`, `text!`, `constraint!`, `constraints!`.
- `utils.rs` provides: `ScrollState`, `WidgetList`, `Separator`, `CowStr`, `Either` (`Left`/`Right` widget variants).
- Inline `impl Widget for &X` pattern for headers and small widgets.

## Current State

- **Mock data** loads in a background thread on startup (see the big literal arrays in `HomeData::new`).
- **Real NetworkManager backend not wired yet** — intended path is `zbus` D-Bus with `nmcli` fallback.
- Only the **Home** pane exists; config editing, VPN, and monitoring are roadmap items.

## Testing

- `src/tests/` — currently has scroll logic tests.
- Run with `cargo test`.

## Conventions for Agents

- Use `anyhow::Result` and `.context("...")` for errors.
- Prefer `Arc<T>` for shared data passed into widgets.
- When adding an overlay, wrap it in `Option`, intercept in `update()` before the pane handler, and draw it on top at the end of `draw()`.
- Keep widget rendering stateless where possible; use `Cell` only for render→update feedback like viewport sizes.
- Match `KeyEventKind::Press` explicitly to avoid double-firing on repeats/releases.
- Use `PATINA` colors; add new semantic colors to `Theme` rather than hardcoding RGBs.
- Log via `log::info!`, `log::warn!`, etc. Logs write to `~/.config/patina/patina.log` by default.
