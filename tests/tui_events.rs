//! Tests headless del manejo de eventos de la TUI (fase 3): navegación,
//! comentario de línea (TS-08) y comentario de rango multilínea (TS-09),
//! inyectando `KeyEvent` al `App` y assertando sobre estado y render.

mod common_tui;

use common_tui::{feed, feed_text, key, key_char, key_ctrl, render_to_string, sample_diff};
use crossterm::event::KeyCode;
use reviewv2::app::{App, Mode};
use reviewv2::review::Side;

/// TS-08: navegar entre archivos y líneas y agregar un comentario de una línea;
/// el comentario queda en el `Review` con el anclaje correcto y se refleja en el
/// render (marcador de comentario en el diff).
#[test]
fn ts08_navigate_and_add_single_line_comment() {
    let mut app = App::new(sample_diff());

    // Enfocar DIFF y mover el cursor hasta la línea añadida `FileStatus` (lado
    // nuevo, número 15). El primer archivo `src/diff.rs` está seleccionado.
    feed(&mut app, &[key_char('2')]);

    // Bajar el cursor hasta posicionarse sobre la línea añadida (4ª línea del
    // hunk: context12, context13, removed14, added15).
    feed(&mut app, &[key_char('j'), key_char('j'), key_char('j')]);

    // Abrir el editor de comentario de línea con `c`.
    feed(&mut app, &[key_char('c')]);
    assert_eq!(
        app.mode(),
        Mode::EditComment,
        "`c` debe entrar en modo de edición de comentario"
    );

    // Escribir el cuerpo y guardar con Ctrl+S.
    feed_text(&mut app, "¿FileStatus cubre renamed?");
    feed(&mut app, &[key_ctrl('s')]);

    // Tras guardar, volvemos a navegación.
    assert_eq!(
        app.mode(),
        Mode::Navigate,
        "tras Ctrl+S debe volver a modo navegación"
    );

    // El comentario quedó en el Review.
    let comments = app.review().comments();
    assert_eq!(comments.len(), 1, "debe haber exactamente un comentario");
    let c = &comments[0];
    assert_eq!(c.file, "src/diff.rs", "anclado al archivo seleccionado");
    assert_eq!(c.side, Side::New, "la línea añadida es del lado nuevo");
    assert_eq!(c.start_line, 15, "anclado a la línea nueva 15");
    assert_eq!(c.end_line, 15, "comentario de una línea: end == start");
    assert_eq!(c.body, "¿FileStatus cubre renamed?");

    // Reflejo en el render: el marcador de comentario `💬` aparece en el diff.
    let screen = render_to_string(&app, 120, 30);
    assert!(
        screen.contains('💬'),
        "el diff debe mostrar el marcador `💬` en la línea comentada, pantalla:\n{screen}"
    );
}

/// TS-08: el editor de comentario puede cancelarse con `Esc` sin agregar nada.
#[test]
fn ts08_esc_cancels_comment_editor_without_adding() {
    let mut app = App::new(sample_diff());
    feed(&mut app, &[key_char('2'), key_char('j')]);
    feed(&mut app, &[key_char('c')]);
    assert_eq!(app.mode(), Mode::EditComment);

    feed_text(&mut app, "borrador descartado");
    feed(&mut app, &[key(KeyCode::Esc)]);

    assert_eq!(app.mode(), Mode::Navigate, "Esc debe volver a navegación");
    assert_eq!(
        app.review().comments().len(),
        0,
        "Esc no debe agregar ningún comentario"
    );
}

/// TS-09: seleccionar un rango de líneas con `v` y agregar un comentario
/// multilínea; queda anclado a un rango `start..end` y se refleja en estado y
/// render.
#[test]
fn ts09_select_range_and_add_multiline_comment() {
    let mut app = App::new(sample_diff());

    // Segundo archivo `src/main.rs`: tiene tres líneas añadidas 20,21,22.
    feed(&mut app, &[key_char('j')]); // archivo -> main.rs (foco FILES)
    assert_eq!(app.selected_file_index(), 1);

    // Enfocar DIFF y posicionar el cursor en la primera añadida (línea nueva 20):
    // hunk = context19, added20, added21, added22 -> bajar 1 desde el tope.
    feed(&mut app, &[key_char('2'), key_char('j')]);

    // Iniciar selección de rango con `v`.
    feed(&mut app, &[key_char('v')]);
    assert_eq!(
        app.mode(),
        Mode::RangeSelect,
        "`v` debe entrar en modo de selección de rango"
    );

    // Extender el rango dos líneas hacia abajo (20 -> 22).
    feed(&mut app, &[key_char('j'), key_char('j')]);

    // Confirmar el rango para comentar con `c`.
    feed(&mut app, &[key_char('c')]);
    assert_eq!(
        app.mode(),
        Mode::EditComment,
        "`c` sobre un rango activo debe abrir el editor de comentario"
    );

    feed_text(&mut app, "Extraer este loop a una función con test propio.");
    feed(&mut app, &[key_ctrl('s')]);

    assert_eq!(app.mode(), Mode::Navigate);

    let comments = app.review().comments();
    assert_eq!(comments.len(), 1, "un comentario de rango");
    let c = &comments[0];
    assert_eq!(c.file, "src/main.rs", "anclado a main.rs");
    assert_eq!(c.side, Side::New, "añadidas -> lado nuevo");
    assert_eq!(c.start_line, 20, "inicio del rango");
    assert_eq!(c.end_line, 22, "fin del rango");
    assert_eq!(c.body, "Extraer este loop a una función con test propio.");

    // Reflejo en el render: marcador de comentario presente.
    let screen = render_to_string(&app, 120, 30);
    assert!(
        screen.contains('💬'),
        "el diff debe mostrar el marcador `💬` en el rango comentado, pantalla:\n{screen}"
    );
}

/// TS-09: el modo de selección de rango se cancela con `Esc` volviendo a
/// navegación sin agregar comentario.
#[test]
fn ts09_esc_cancels_range_selection() {
    let mut app = App::new(sample_diff());
    feed(&mut app, &[key_char('j'), key_char('2'), key_char('j')]);
    feed(&mut app, &[key_char('v')]);
    assert_eq!(app.mode(), Mode::RangeSelect);

    feed(&mut app, &[key(KeyCode::Esc)]);
    assert_eq!(
        app.mode(),
        Mode::Navigate,
        "Esc debe cancelar la selección de rango"
    );
    assert_eq!(app.review().comments().len(), 0);
}
