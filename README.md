# Vibe Todo

A todo window that lives on your macOS desk. Native, written in Rust on
[gpui](https://www.gpui.rs/) — not a web page in a wrapper.

There is one column, split into two. "进行中" at the top holds what you are
actually working on right now; "收件箱" underneath is where everything else
lands as you think of it. To start something, drag it up.

> The interface is in Simplified Chinese, as you can see in the screenshots
> above. Everything else here — code, comments, this file — is in English.

## The idea

- **One sheet of paper.** The whole window is a single warm off-white, running
  from the title bar to the bottom edge with no second surface, so empty space
  reads as room rather than as something unfinished. The two sections are told
  apart by a heading and a hairline; both scroll together, as one sheet. Only
  the completed-tasks popover and the card under the cursor during a drag leave
  that plane, and they do it by casting a shadow.
- **Two colours, one job each.** Blue means done, red means priority. Nothing
  else gets a hue.
- **Checking something off takes two steps.** The row is struck through and
  holds still for 240ms — click again in that window and it comes back — then it
  folds shut, and only then is the change written. A misclick has somewhere to
  go.
- **Actions hide until you reach for them.** Hover a row and a tray slides in
  from the right; the title dissolves under it rather than being cut off. Nothing
  sits there permanently taking up space.
- **Destructive buttons need two clicks.** "Clear completed" is armed by the
  first click and disarmed automatically three seconds later, so a click that
  lands minutes afterwards cannot finish something you started by accident.

## Keyboard

| Shortcut | Action |
| --- | --- |
| `⌘N` | New task |
| `⌘⇧P` | Keep the window on top |
| `⌘⇧H` | Show / hide completed |
| `⌘⌫` | Delete the task under the pointer |
| `Esc` | Cancel the current input |

`⌘⌫` is deliberately not bound while you are editing a title — there it means
"delete to the start of the line", and stealing it would delete the task instead.

## What it remembers

Window position, size, and whether it is pinned are all stored, so it reopens
the way you left it. Geometry saved on a display that is no longer attached is
thrown away and the window returns to the centre of the screen — otherwise it
would sit somewhere you can neither see nor drag it back from.

Tasks live in SQLite:

```
~/Library/Application Support/vibe-todo/todo.db
```

If `dirs::data_dir()` is unavailable it falls back to a `todo.db` next to the
binary. WAL is on, so you will also see `-wal` and `-shm` files alongside it.

## Build and run

Requires macOS and a stable Rust toolchain (developed on 1.97, edition 2021).

```bash
cargo run --release
```

To get the app itself rather than a bare binary — a Dock icon, a name in the
menu bar, and the window privileges macOS only grants a bundle:

```bash
./packaging/bundle.sh
```

It writes `target/release/bundle/Vibe Todo.app`, ad-hoc signed so it launches
on Apple silicon. Drag it to `/Applications`. The icon is drawn by
`packaging/make-icon.py` and packed by `packaging/make-icon.sh`; the bundle
script re-runs them only when the drawing is newer than `AppIcon.icns`.

Pinning uses `NSFloatingWindowLevel` through cocoa/objc directly. That part is
compiled per target — it is a no-op off macOS — but the app as a whole has only
been exercised on macOS.

## Tests

```bash
cargo test
```

The data-layer tests run against an in-memory database and never touch your real
todos.

## Layout

| File | What is in it |
| --- | --- |
| `src/app.rs` | The entire interface: drag-to-reorder, the completion animation, the hover tray |
| `src/db.rs` | SQLite access, schema and migration, window state |
| `src/model.rs` | `Task`, `ColumnType`, `WindowState` |
| `src/theme.rs` | Colours and shadows, each with the reasoning next to it |
| `src/icons.rs` | Inline SVG icons |
| `src/window_level.rs` | macOS window level (pinning) |

## License

MIT — see [LICENSE](LICENSE).
