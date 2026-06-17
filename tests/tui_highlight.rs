//! Test de aceptación EN ROJO para la integración del resaltado de sintaxis en
//! la TUI (`reviewv2::app`). HL-05.
//!
//! Contrato de color FIJADO (el coder debe respetarlo EXACTAMENTE):
//!   - keyword PHP `return`  → `.fg == Color::Magenta`
//!   - línea AÑADIDA         → tinte de FONDO `.bg == Color::Green`
//!
//! Se construye un `TestBackend` + `Terminal` propio para poder leer ESTILOS
//! (`.symbol()`, `.fg`, `.bg`) del buffer, no solo símbolos.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

use reviewv2::app::App;
use reviewv2::diff::{Diff, FileDiff, FileStatus, Hunk, Line, LineKind};

use std::path::PathBuf;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 30;

/// Diff sintético de UN archivo PHP con una única línea AÑADIDA cuyo contenido
/// contiene el keyword PHP `return`.
fn php_added_diff() -> Diff {
    let file = FileDiff {
        path: PathBuf::from("src/ejemplo.php"),
        status: FileStatus::Modified,
        additions: 1,
        deletions: 0,
        hunks: vec![Hunk {
            old_start: 9,
            old_lines: 1,
            new_start: 9,
            new_lines: 2,
            lines: vec![
                Line {
                    kind: LineKind::Context,
                    old_lineno: Some(9),
                    new_lineno: Some(9),
                    content: "function f() {".to_owned(),
                },
                Line {
                    kind: LineKind::Added,
                    old_lineno: None,
                    new_lineno: Some(10),
                    content: "    return $x;".to_owned(),
                },
            ],
        }],
    };
    Diff { files: vec![file] }
}

/// Renderiza el `App` a un `TestBackend` propio y devuelve el buffer (clonado)
/// para inspeccionar estilos por celda.
fn render_buffer(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("crear Terminal con TestBackend");
    terminal
        .draw(|frame| app.render(frame))
        .expect("draw del App");
    terminal.backend().buffer().clone()
}

/// Devuelve la cadena de símbolos de la fila `y`.
fn row_string(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..WIDTH {
        s.push_str(buffer[(x, y)].symbol());
    }
    s
}

/// Localiza `(x, y)` de la PRIMERA celda donde empieza el substring `needle`.
fn find_cell(buffer: &ratatui::buffer::Buffer, needle: &str) -> Option<(u16, u16)> {
    for y in 0..HEIGHT {
        let row = row_string(buffer, y);
        if let Some(byte_idx) = row.find(needle) {
            // El render usa caracteres ASCII de ancho 1 en esta zona, así que
            // el índice de byte coincide con la columna de celda.
            let col = row[..byte_idx].chars().count() as u16;
            return Some((col, y));
        }
    }
    None
}

/// HL-05: el código añadido se renderiza con el keyword resaltado y la línea
/// añadida con tinte de fondo. Usamos la vista unificada (columna única) para
/// que el código aparezca contiguo y sus celdas sean inspeccionables.
#[test]
fn hl05_php_added_line_highlights_keyword_and_tints_background() {
    let mut app = App::new(php_added_diff());

    // Enfocamos el panel DIFF y pasamos a vista unificada (columna única).
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('2'),
        crossterm::event::KeyModifiers::NONE,
    ));
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('t'),
        crossterm::event::KeyModifiers::NONE,
    ));

    let buffer = render_buffer(&app);

    // (a) El texto del código aparece EXACTO en alguna fila del buffer.
    let cell = find_cell(&buffer, "return $x;");
    assert!(
        cell.is_some(),
        "alguna fila del buffer debe contener `return $x;` exacto; buffer:\n{}",
        (0..HEIGHT)
            .map(|y| row_string(&buffer, y))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let (kw_x, kw_y) = cell.expect("celda de `return $x;`");

    // La celda localizada empieza en la `r` de `return`.
    assert_eq!(
        buffer[(kw_x, kw_y)].symbol(),
        "r",
        "la celda localizada debe ser la `r` inicial de `return`"
    );

    // (b) La celda donde empieza el keyword `return` tiene `.fg` resaltado y,
    // específicamente, el color FIJADO `Color::Magenta`.
    let kw_fg = buffer[(kw_x, kw_y)].fg;
    assert_ne!(
        kw_fg,
        Color::Reset,
        "la celda del keyword `return` no debe tener el fg por defecto (Reset)"
    );
    assert_eq!(
        kw_fg,
        Color::Magenta,
        "el keyword `return` debe colorearse con Color::Magenta (color fijado por el contrato)"
    );

    // (c) La fila de la línea añadida debe tener tinte de FONDO: alguna celda de
    // esa fila con `.bg == Color::Green` (color de fondo fijado para 'añadido').
    let added_bg_present = (0..WIDTH).any(|x| buffer[(x, kw_y)].bg == Color::Green);
    assert!(
        added_bg_present,
        "la línea añadida debe tener tinte de fondo Color::Green en al menos una celda de su fila"
    );

    // La celda del keyword también debe llevar el fondo de 'añadido' (el tinte
    // de fondo cubre la línea, incluido el código resaltado).
    assert_eq!(
        buffer[(kw_x, kw_y)].bg,
        Color::Green,
        "la celda del keyword resaltado debe conservar el tinte de fondo Color::Green de la línea añadida"
    );
}
