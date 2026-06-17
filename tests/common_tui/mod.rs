//! Helpers de test para la TUI (fase 3). Construyen un `Diff` sintético a mano
//! (sin git real) para tests deterministas del render y del manejo de eventos,
//! y exponen utilidades para inyectar `KeyEvent` y leer el buffer de
//! `TestBackend`.
//!
//! Vive bajo `tests/` (no es producción).
#![allow(dead_code)]

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use reviewv2::app::App;
use reviewv2::diff::{Diff, FileDiff, FileStatus, Hunk, Line, LineKind};

/// Construye una línea de contexto (presente en ambos lados).
pub fn context(old: u32, new: u32, content: &str) -> Line {
    Line {
        kind: LineKind::Context,
        old_lineno: Some(old),
        new_lineno: Some(new),
        content: content.to_owned(),
    }
}

/// Construye una línea eliminada (solo lado viejo).
pub fn removed(old: u32, content: &str) -> Line {
    Line {
        kind: LineKind::Removed,
        old_lineno: Some(old),
        new_lineno: None,
        content: content.to_owned(),
    }
}

/// Construye una línea añadida (solo lado nuevo).
pub fn added(new: u32, content: &str) -> Line {
    Line {
        kind: LineKind::Added,
        old_lineno: None,
        new_lineno: Some(new),
        content: content.to_owned(),
    }
}

/// Diff sintético con dos archivos modificados, hunks y líneas deterministas.
///
/// Archivo 1: `src/diff.rs` — contexto, una eliminada y una añadida (para
/// ejercitar split OLD/NEW y comentarios de línea).
/// Archivo 2: `src/main.rs` — varias añadidas consecutivas (para rango).
pub fn sample_diff() -> Diff {
    let file1 = FileDiff {
        path: PathBuf::from("src/diff.rs"),
        status: FileStatus::Modified,
        additions: 1,
        deletions: 1,
        hunks: vec![Hunk {
            old_start: 12,
            old_lines: 4,
            new_start: 12,
            new_lines: 4,
            lines: vec![
                context(12, 12, "pub struct FileDiff {"),
                context(13, 13, "    pub path: PathBuf,"),
                removed(14, "    pub status: u8,"),
                added(15, "    pub status: FileStatus,"),
                context(15, 16, "    pub hunks: Vec<Hunk>,"),
            ],
        }],
    };

    let file2 = FileDiff {
        path: PathBuf::from("src/main.rs"),
        status: FileStatus::Modified,
        additions: 3,
        deletions: 0,
        hunks: vec![Hunk {
            old_start: 19,
            old_lines: 1,
            new_start: 20,
            new_lines: 4,
            lines: vec![
                context(19, 19, "fn run() {"),
                added(20, "    let mut files = Vec::new();"),
                added(21, "    for hunk in raw {"),
                added(22, "        files.push(hunk);"),
            ],
        }],
    };

    Diff {
        files: vec![file1, file2],
    }
}

/// Diff vacío (working tree limpio).
pub fn empty_diff() -> Diff {
    Diff { files: vec![] }
}

/// Atajo para un `KeyEvent` desde un carácter (sin modificadores).
pub fn key_char(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

/// `KeyEvent` desde un `KeyCode` arbitrario (Tab, flechas, Esc, Enter).
pub fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// `Ctrl+<c>`.
pub fn key_ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// Inyecta una secuencia de teclas en el `App`, devolviendo el último `Outcome`.
pub fn feed(app: &mut App, keys: &[KeyEvent]) -> reviewv2::app::Outcome {
    let mut last = reviewv2::app::Outcome::Continue;
    for k in keys {
        last = app.handle_key(*k);
    }
    last
}

/// Inyecta una cadena carácter por carácter como teclas (para el editor de
/// comentario / comentario general).
pub fn feed_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        app.handle_key(key_char(ch));
    }
}

/// Renderiza el `App` a un `TestBackend` y devuelve las filas del buffer como
/// strings (una por fila), concatenando el símbolo de cada celda.
pub fn render_to_rows(app: &App, width: u16, height: u16) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("crear Terminal con TestBackend");
    terminal
        .draw(|frame| app.render(frame))
        .expect("draw del App");

    let buffer = terminal.backend().buffer().clone();
    let area = *buffer.area();
    let mut rows = Vec::with_capacity(area.height as usize);
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            line.push_str(buffer[(x, y)].symbol());
        }
        rows.push(line);
    }
    rows
}

/// Renderiza y devuelve TODO el buffer como un único string (filas unidas por
/// `\n`). Útil para asserts de presencia de substring.
pub fn render_to_string(app: &App, width: u16, height: u16) -> String {
    render_to_rows(app, width, height).join("\n")
}

/// ¿Alguna fila del render contiene el substring dado?
pub fn any_row_contains(rows: &[String], needle: &str) -> bool {
    rows.iter().any(|r| r.contains(needle))
}
