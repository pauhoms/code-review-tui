//! Tests de aceptación EN ROJO para el "lado activo" (OLD/NEW) en modo SPLIT.
//!
//! Contrato asumido (el coder lo materializa):
//!   - Getter nuevo: `App::active_side(&self) -> reviewv2::review::Side`.
//!     Arranca en `Side::New`.
//!   - En modo Navigate (y RangeSelect) con foco DIFF:
//!       `h` / `KeyCode::Left`  → `active_side = Side::Old`
//!       `l` / `KeyCode::Right` → `active_side = Side::New`
//!   - Anclaje al comentar:
//!       Removed → siempre `Side::Old` (old_lineno).
//!       Added   → siempre `Side::New` (new_lineno).
//!       Context → al lado activo (Old=old_lineno, New=new_lineno).
//!   - Render split: el cursor (Modifier::REVERSED) se resalta SOLO en la
//!     columna del lado activo.
#![allow(clippy::doc_overindented_list_items)]

mod common_tui;

use common_tui::{feed, feed_text, key, key_char, key_ctrl, sample_diff};

use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

use reviewv2::app::App;
use reviewv2::review::Side;

/// SP-01: `active_side()` arranca en `Side::New`; con foco DIFF, `h`/`←` lo
/// ponen en `Side::Old` y `l`/`→` lo devuelven a `Side::New`.
#[test]
fn sp01_active_side_starts_new_and_toggles_with_hl_and_arrows() {
    let mut app = App::new(sample_diff());

    assert_eq!(
        app.active_side(),
        Side::New,
        "el lado activo debe arrancar en Side::New"
    );

    // Foco DIFF.
    feed(&mut app, &[key_char('2')]);

    // `h` → OLD
    feed(&mut app, &[key_char('h')]);
    assert_eq!(
        app.active_side(),
        Side::Old,
        "`h` debe poner el lado activo en Side::Old"
    );

    // `l` → NEW
    feed(&mut app, &[key_char('l')]);
    assert_eq!(
        app.active_side(),
        Side::New,
        "`l` debe poner el lado activo en Side::New"
    );

    // `←` (Left) → OLD
    feed(&mut app, &[key(KeyCode::Left)]);
    assert_eq!(
        app.active_side(),
        Side::Old,
        "`KeyCode::Left` debe poner el lado activo en Side::Old"
    );

    // `→` (Right) → NEW
    feed(&mut app, &[key(KeyCode::Right)]);
    assert_eq!(
        app.active_side(),
        Side::New,
        "`KeyCode::Right` debe poner el lado activo en Side::New"
    );
}

/// SP-02 (lado OLD): comentar una línea de CONTEXTO (índice 4: old=15/new=16)
/// con lado activo OLD ancla a (src/diff.rs, Side::Old, 15..15).
#[test]
fn sp02_context_comment_anchors_to_old_side_when_active_old() {
    let mut app = App::new(sample_diff());

    // Foco DIFF, mover el cursor al índice 4 (4 pulsaciones de `j` desde 0).
    feed(
        &mut app,
        &[
            key_char('2'),
            key_char('j'),
            key_char('j'),
            key_char('j'),
            key_char('j'),
        ],
    );
    assert_eq!(app.cursor_line(), 4, "el cursor debe estar en el índice 4");

    // Lado activo OLD.
    feed(&mut app, &[key_char('h')]);
    assert_eq!(app.active_side(), Side::Old);

    // Comentar.
    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "comentario en contexto lado old");
    feed(&mut app, &[key_ctrl('s')]);

    let comments = app.review().comments();
    assert_eq!(comments.len(), 1, "debe haber exactamente un comentario");
    let c = &comments[0];
    assert_eq!(c.file, "src/diff.rs", "el archivo del anclaje");
    assert_eq!(
        c.side,
        Side::Old,
        "con lado activo OLD, el contexto se ancla a Side::Old"
    );
    assert_eq!(
        c.start_line, 15,
        "con lado activo OLD el contexto usa old_lineno=15"
    );
    assert_eq!(c.end_line, 15, "comentario de una sola línea: end == start");
}

/// SP-02 (lado NEW): la misma línea de CONTEXTO con lado activo NEW ancla a
/// (Side::New, 16..16).
#[test]
fn sp02_context_comment_anchors_to_new_side_when_active_new() {
    let mut app = App::new(sample_diff());

    feed(
        &mut app,
        &[
            key_char('2'),
            key_char('j'),
            key_char('j'),
            key_char('j'),
            key_char('j'),
        ],
    );
    assert_eq!(app.cursor_line(), 4, "el cursor debe estar en el índice 4");

    // Lado activo NEW (es el inicial, pero lo fijamos explícitamente).
    feed(&mut app, &[key_char('l')]);
    assert_eq!(app.active_side(), Side::New);

    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "comentario en contexto lado new");
    feed(&mut app, &[key_ctrl('s')]);

    let comments = app.review().comments();
    assert_eq!(comments.len(), 1, "debe haber exactamente un comentario");
    let c = &comments[0];
    assert_eq!(c.file, "src/diff.rs", "el archivo del anclaje");
    assert_eq!(
        c.side,
        Side::New,
        "con lado activo NEW, el contexto se ancla a Side::New"
    );
    assert_eq!(
        c.start_line, 16,
        "con lado activo NEW el contexto usa new_lineno=16"
    );
    assert_eq!(c.end_line, 16, "comentario de una sola línea: end == start");
}

/// SP-03 (Added): la línea AÑADIDA (índice 3) se ancla siempre a Side::New/15
/// aunque el lado activo sea OLD.
#[test]
fn sp03_added_line_always_anchors_new_even_when_active_old() {
    let mut app = App::new(sample_diff());

    // Foco DIFF, cursor al índice 3 (3 pulsaciones de `j`).
    feed(
        &mut app,
        &[key_char('2'), key_char('j'), key_char('j'), key_char('j')],
    );
    assert_eq!(app.cursor_line(), 3, "el cursor debe estar en el índice 3");

    // Forzamos lado activo OLD.
    feed(&mut app, &[key_char('h')]);
    assert_eq!(app.active_side(), Side::Old);

    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "comentario en línea añadida");
    feed(&mut app, &[key_ctrl('s')]);

    let comments = app.review().comments();
    assert_eq!(comments.len(), 1, "debe haber exactamente un comentario");
    let c = &comments[0];
    assert_eq!(c.file, "src/diff.rs");
    assert_eq!(
        c.side,
        Side::New,
        "una línea Added se ancla SIEMPRE a Side::New aunque el lado activo sea OLD"
    );
    assert_eq!(c.start_line, 15, "una línea Added usa new_lineno=15");
    assert_eq!(c.end_line, 15);
}

/// SP-03 (Removed): la línea ELIMINADA (índice 2) se ancla siempre a
/// Side::Old/14 aunque el lado activo sea NEW.
#[test]
fn sp03_removed_line_always_anchors_old_even_when_active_new() {
    let mut app = App::new(sample_diff());

    // Foco DIFF, cursor al índice 2 (2 pulsaciones de `j`).
    feed(&mut app, &[key_char('2'), key_char('j'), key_char('j')]);
    assert_eq!(app.cursor_line(), 2, "el cursor debe estar en el índice 2");

    // Forzamos lado activo NEW (inicial, pero explícito).
    feed(&mut app, &[key_char('l')]);
    assert_eq!(app.active_side(), Side::New);

    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "comentario en línea eliminada");
    feed(&mut app, &[key_ctrl('s')]);

    let comments = app.review().comments();
    assert_eq!(comments.len(), 1, "debe haber exactamente un comentario");
    let c = &comments[0];
    assert_eq!(c.file, "src/diff.rs");
    assert_eq!(
        c.side,
        Side::Old,
        "una línea Removed se ancla SIEMPRE a Side::Old aunque el lado activo sea NEW"
    );
    assert_eq!(c.start_line, 14, "una línea Removed usa old_lineno=14");
    assert_eq!(c.end_line, 14);
}

const WIDTH: u16 = 120;
const HEIGHT: u16 = 30;

fn render_buffer(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("crear Terminal con TestBackend");
    terminal
        .draw(|frame| app.render(frame))
        .expect("draw del App");
    terminal.backend().buffer().clone()
}

fn row_string(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..WIDTH {
        s.push_str(buffer[(x, y)].symbol());
    }
    s
}

/// Localiza la fila `y` del cursor: la fila que contiene el contenido de la
/// línea de contexto del índice 4 (`pub hunks`).
fn cursor_row(buffer: &ratatui::buffer::Buffer) -> u16 {
    for y in 0..HEIGHT {
        if row_string(buffer, y).contains("pub hunks") {
            return y;
        }
    }
    panic!(
        "no se encontró la fila del cursor (`pub hunks`); buffer:\n{}",
        (0..HEIGHT)
            .map(|y| row_string(buffer, y))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// ¿Alguna celda de la columna IZQUIERDA (x < WIDTH/2) de la fila `y` lleva
/// `Modifier::REVERSED`?
fn left_has_reversed(buffer: &ratatui::buffer::Buffer, y: u16) -> bool {
    (0..WIDTH / 2).any(|x| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
}

/// ¿Alguna celda de la columna DERECHA (x >= WIDTH/2) de la fila `y` lleva
/// `Modifier::REVERSED`?
fn right_has_reversed(buffer: &ratatui::buffer::Buffer, y: u16) -> bool {
    (WIDTH / 2..WIDTH).any(|x| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
}

/// SP-04: en vista split, el resaltado del cursor (REVERSED) aparece solo en la
/// columna del lado activo: NEW → derecha; al cambiar a OLD (`h`) → izquierda.
#[test]
fn sp04_split_cursor_highlight_only_in_active_side_column() {
    let mut app = App::new(sample_diff());

    // Foco DIFF, cursor al índice 4 (fila de contexto presente en ambas columnas).
    feed(
        &mut app,
        &[
            key_char('2'),
            key_char('j'),
            key_char('j'),
            key_char('j'),
            key_char('j'),
        ],
    );
    assert_eq!(app.cursor_line(), 4);

    // --- Lado activo NEW (inicial): REVERSED en la columna DERECHA, no en la izquierda. ---
    assert_eq!(app.active_side(), Side::New);
    let buffer = render_buffer(&app);
    let y = cursor_row(&buffer);

    assert!(
        right_has_reversed(&buffer, y),
        "con lado activo NEW, la columna DERECHA de la fila del cursor debe tener REVERSED"
    );
    assert!(
        !left_has_reversed(&buffer, y),
        "con lado activo NEW, la columna IZQUIERDA de la fila del cursor NO debe tener REVERSED"
    );

    // --- Cambiar a OLD (`h`): se invierte. ---
    feed(&mut app, &[key_char('h')]);
    assert_eq!(app.active_side(), Side::Old);
    let buffer = render_buffer(&app);
    let y = cursor_row(&buffer);

    assert!(
        left_has_reversed(&buffer, y),
        "con lado activo OLD, la columna IZQUIERDA de la fila del cursor debe tener REVERSED"
    );
    assert!(
        !right_has_reversed(&buffer, y),
        "con lado activo OLD, la columna DERECHA de la fila del cursor NO debe tener REVERSED"
    );
}
