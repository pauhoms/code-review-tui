# code-review-tui

A terminal code reviewer with the mental flow of a GitHub pull request review,
for the **uncommitted** changes of any git repo. Written in Rust with
[ratatui](https://ratatui.rs).

Show the working-tree diff, leave comments anchored to lines or ranges, write an
overall comment, pick an **LGTM** / **KO** verdict and, when you finish, generate
a PR-style Markdown report with everything above.

## Features

- **Diff of uncommitted changes** against `HEAD`: staged + unstaged + untracked +
  deletions, in a single change set (via `git2`/libgit2).
- **Side-by-side (split) view by default** with OLD | NEW columns, or **unified**
  with the `t` key.
- **Switchable active side** in split (`h`/`l` or `←`/`→`): the cursor is
  highlighted on the chosen column and comments anchor to that side.
- **Comments** on a single line (`c`) or a multi-line **range** (`v`), anchored
  to `file:line` / `file:Lstart-Lend`.
- **Syntax highlighting** for PHP and TypeScript.
- **Numbered, focusable panels**: `[1]` FILES, `[2]` DIFF, `[3]` comment thread.
- **Final screen** with the comment summary, the overall comment and the
  verdict; on save it writes `code-review-<date>.md` to the current directory.

## Installation

Requires a Rust toolchain (edition 2024; tested with 1.95).

```bash
git clone git@github.com:pauhoms/code-review-tui.git
cd code-review-tui
cargo build --release
```

The binary lands at `target/release/reviewv2`.

## Usage

Run it inside a git repo with uncommitted changes:

```bash
cd /path/to/your/repo
/path/to/code-review-tui/target/release/reviewv2
```

If there are no changes, it shows an empty state and exits with `q`. When you
finish a review, the Markdown report is written to the current directory.

## Keybindings

| Key | Action |
|---|---|
| `1` / `2` | Focus the FILES / DIFF panel |
| `Tab` / `Shift+Tab` | Cycle focus between panels |
| `j` / `k` | Move (file when in FILES, line when in DIFF) |
| `h` / `l` · `←` / `→` | Switch the active side (OLD / NEW) in split |
| `t` | Toggle split / unified view |
| `c` | Comment the line under the cursor |
| `v` | Start a range selection (`j`/`k` extends, `c` comments) |
| `↵` | Open the comment thread for the line |
| `g` | Final screen (overall comment + verdict) |
| `Ctrl+S` | Save comment / finish and write the report |
| `Esc` | Cancel / go back |
| `q` | Quit |

On the final screen: `↑`/`↓` walks the comments, `←`/`→` picks the verdict
(LGTM / KO) and `↵` jumps to the thread of the selected comment.

## Architecture

Three decoupled, testable layers:

1. **`diff`** — acquires the uncommitted diff with `git2` and builds a structured
   model (files → hunks → typed lines with old/new line numbers).
2. **`review`** — pure model of the review (comments, overall comment, verdict)
   and its deterministic Markdown serialization, kept separate from disk I/O.
3. **`app`** — the ratatui TUI that orchestrates the previous two; all state and
   event handling is drivable *headless* so it can be tested.

## Tests

```bash
cargo test
```

The TUI is tested without a real terminal using `ratatui::backend::TestBackend`
(rendering to a cell buffer) and injecting keyboard events; the diff and review
layers are tested with temporary git repositories and string comparison.

## License

[MIT](LICENSE)
