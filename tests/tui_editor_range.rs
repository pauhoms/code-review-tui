//! Visualización del editor de comentario (modo EditComment) y del rango activo
//! (modo RangeSelect) — mejoras de UX sobre la Fase 3.

mod common_tui;

use common_tui::{feed, feed_text, key_char, render_to_string, sample_diff};
use reviewv2::app::App;

/// Al redactar un comentario, la caja del editor muestra el anclaje, el texto
/// que se va escribiendo y la ayuda de teclas.
#[test]
fn edit_comment_box_shows_anchor_typed_text_and_hints() {
    let mut app = App::new(sample_diff());
    // Enfocar DIFF e ir a la línea añadida 15 (índice 3).
    feed(
        &mut app,
        &[key_char('2'), key_char('j'), key_char('j'), key_char('j')],
    );
    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "necesita revisión");

    let screen = render_to_string(&app, 120, 30);

    assert!(
        screen.contains("necesita revisión"),
        "el editor debe mostrar el texto que se está escribiendo, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("src/diff.rs:15"),
        "el editor debe mostrar el anclaje del comentario, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("Ctrl+S") && screen.contains("Esc"),
        "el editor debe mostrar la ayuda de teclas (Ctrl+S / Esc), pantalla:\n{screen}"
    );
}

/// En selección de rango (split) las líneas seleccionadas se resaltan con un
/// marcador de canaleta y la barra indica el modo y cuántas líneas llevás.
#[test]
fn range_select_highlights_selection_and_shows_count_split() {
    let mut app = App::new(sample_diff());
    // Foco DIFF, cursor en índice 1, iniciar rango y extender a índice 3 (3 líneas).
    feed(&mut app, &[key_char('2'), key_char('j')]);
    feed(&mut app, &[key_char('v'), key_char('j'), key_char('j')]);

    let screen = render_to_string(&app, 120, 30);

    assert!(
        screen.contains('▌'),
        "las líneas seleccionadas deben mostrar el marcador de canaleta `▌`, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("RANGO") && screen.contains("3 líneas"),
        "la barra debe indicar el modo RANGO y 3 líneas seleccionadas, pantalla:\n{screen}"
    );
}

/// El resaltado del rango también funciona en la vista unificada (se muestra en
/// el modo en el que estás).
#[test]
fn range_select_highlights_selection_in_unified() {
    let mut app = App::new(sample_diff());
    // Foco DIFF, pasar a unificado, cursor en índice 1, rango hasta índice 3.
    feed(&mut app, &[key_char('2'), key_char('t'), key_char('j')]);
    feed(&mut app, &[key_char('v'), key_char('j'), key_char('j')]);

    let screen = render_to_string(&app, 120, 30);

    assert!(
        screen.contains('▌'),
        "en unificado las líneas seleccionadas deben resaltarse con `▌`, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("3 líneas"),
        "la barra debe indicar 3 líneas seleccionadas en unificado, pantalla:\n{screen}"
    );
}
