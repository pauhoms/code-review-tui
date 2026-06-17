//! Tests de aceptación de los criterios NUEVOS de la Fase 3 (sub-tareas de UX):
//! TS-15 (abrir/cerrar panel `[3]` hilo del comentario de la línea bajo el
//! cursor con `↵`/`Esc`), TS-16 (ciclado de foco FILES↔DIFF con `Tab`/`BackTab`)
//! y TS-17 (navegar y saltar a comentarios desde la pantalla final).
//!
//! Headless con `TestBackend` + inyección de `KeyEvent`. No tocan producción ni
//! reutilizan más que el helper común `common_tui`.

mod common_tui;

use common_tui::{feed, feed_text, key, key_char, key_ctrl, render_to_string, sample_diff};
use crossterm::event::KeyCode;
use reviewv2::app::{App, Focus, Mode};

/// Móntar un comentario de línea en `src/diff.rs:15` (índice aplanado 3 del
/// primer archivo) dejando el cursor sobre esa misma línea, en modo navegación.
fn app_with_comment_on_diff_line_15() -> App {
    let mut app = App::new(sample_diff());
    // Enfocar DIFF y bajar hasta la línea añadida 15 (índice 3: ctx12, ctx13,
    // removed14, added15).
    feed(&mut app, &[key_char('2')]);
    feed(&mut app, &[key_char('j'), key_char('j'), key_char('j')]);
    // Comentar esa línea.
    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "cuerpo del hilo en la linea 15");
    feed(&mut app, &[key_ctrl('s')]);
    // Tras guardar, el cursor sigue en el índice 3 y estamos en navegación.
    assert_eq!(app.mode(), Mode::Navigate);
    assert_eq!(app.focus(), Focus::Diff);
    assert_eq!(app.cursor_line(), 3, "el cursor debe quedar en la linea 15");
    assert_eq!(
        app.review().comments().len(),
        1,
        "precondición: un comentario montado"
    );
    app
}

/// TS-15: con foco en DIFF sobre una línea YA comentada, `↵` abre el panel `[3]`
/// hilo: el foco pasa a `Focus::Thread` y el render muestra el literal `[3]` y el
/// CUERPO del comentario.
#[test]
fn ts15_enter_on_commented_line_opens_thread_panel_with_body() {
    let mut app = app_with_comment_on_diff_line_15();

    // `↵` sobre la línea comentada abre el hilo.
    feed(&mut app, &[key(KeyCode::Enter)]);
    assert_eq!(
        app.focus(),
        Focus::Thread,
        "`↵` sobre una línea comentada debe pasar el foco a Thread"
    );

    let screen = render_to_string(&app, 120, 30);
    assert!(
        screen.contains("[3]"),
        "el panel de hilo debe mostrar el literal `[3]`, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("cuerpo del hilo en la linea 15"),
        "el panel de hilo debe mostrar el CUERPO del comentario, pantalla:\n{screen}"
    );
}

/// TS-15: con el hilo abierto, `Esc` lo cierra y el foco vuelve a `Focus::Diff`.
#[test]
fn ts15_esc_closes_thread_and_returns_focus_to_diff() {
    let mut app = app_with_comment_on_diff_line_15();

    feed(&mut app, &[key(KeyCode::Enter)]);
    assert_eq!(app.focus(), Focus::Thread, "precondición: hilo abierto");

    feed(&mut app, &[key(KeyCode::Esc)]);
    assert_eq!(
        app.focus(),
        Focus::Diff,
        "`Esc` debe cerrar el hilo y devolver el foco a Diff"
    );
}

/// TS-15: `↵` sobre una línea SIN comentario NO abre el hilo (el foco sigue en
/// Diff). Evita falsos positivos del comportamiento de apertura.
#[test]
fn ts15_enter_on_uncommented_line_does_not_open_thread() {
    let mut app = App::new(sample_diff());
    // Enfocar DIFF; el cursor arranca en el índice 0 (línea de contexto 12, sin
    // comentario).
    feed(&mut app, &[key_char('2')]);
    assert_eq!(app.cursor_line(), 0);
    assert_eq!(app.review().comments().len(), 0);

    feed(&mut app, &[key(KeyCode::Enter)]);
    assert_eq!(
        app.focus(),
        Focus::Diff,
        "`↵` sobre una línea sin comentario no debe abrir el hilo"
    );
}

/// TS-16: `Tab` cicla el foco FILES → DIFF → FILES (el panel THREAD no entra en
/// el ciclo mientras no haya hilo abierto).
#[test]
fn ts16_tab_cycles_focus_files_diff_files() {
    let mut app = App::new(sample_diff());
    assert_eq!(app.focus(), Focus::Files, "estado inicial: foco en Files");

    feed(&mut app, &[key(KeyCode::Tab)]);
    assert_eq!(app.focus(), Focus::Diff, "primer `Tab`: Files → Diff");

    feed(&mut app, &[key(KeyCode::Tab)]);
    assert_eq!(
        app.focus(),
        Focus::Files,
        "segundo `Tab`: Diff → Files (ciclo de dos paneles)"
    );
}

/// TS-16: `Shift+Tab` (`BackTab`) cicla el foco en sentido inverso, que para un
/// ciclo de dos paneles es FILES → DIFF → FILES.
#[test]
fn ts16_backtab_cycles_focus_in_reverse() {
    let mut app = App::new(sample_diff());
    assert_eq!(app.focus(), Focus::Files, "estado inicial: foco en Files");

    feed(&mut app, &[key(KeyCode::BackTab)]);
    assert_eq!(
        app.focus(),
        Focus::Diff,
        "primer `BackTab`: Files → Diff (inverso sobre dos paneles)"
    );

    feed(&mut app, &[key(KeyCode::BackTab)]);
    assert_eq!(app.focus(), Focus::Files, "segundo `BackTab`: Diff → Files");
}

/// Móntar dos comentarios (línea `src/diff.rs:15` y rango `src/main.rs:20-22`),
/// como en el TS-14 existente, y abrir la pantalla final.
fn app_with_two_comments_on_final() -> App {
    let mut app = App::new(sample_diff());

    // Comentario de línea en src/diff.rs:15.
    feed(&mut app, &[key_char('2')]);
    feed(&mut app, &[key_char('j'), key_char('j'), key_char('j')]);
    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "comentario de linea");
    feed(&mut app, &[key_ctrl('s')]);

    // Comentario de rango en src/main.rs:20-22.
    feed(&mut app, &[key_char('1')]); // foco FILES
    feed(&mut app, &[key_char('j')]); // -> main.rs
    feed(&mut app, &[key_char('2'), key_char('j')]); // foco DIFF, cursor en 20
    feed(&mut app, &[key_char('v'), key_char('j'), key_char('j')]); // rango 20..22
    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "comentario de rango");
    feed(&mut app, &[key_ctrl('s')]);

    // Abrir pantalla final.
    feed(&mut app, &[key_char('g')]);
    assert_eq!(
        app.mode(),
        Mode::Final,
        "precondición: pantalla final abierta"
    );
    assert_eq!(
        app.review().comments().len(),
        2,
        "precondición: dos comentarios montados"
    );
    app
}

/// TS-17: en la pantalla final, el comentario seleccionado arranca en el índice 0
/// y `↓`/`↑` mueven la selección entre los comentarios listados.
#[test]
fn ts17_arrows_move_selected_comment_in_final_screen() {
    let mut app = app_with_two_comments_on_final();

    assert_eq!(
        app.selected_comment_index(),
        0,
        "la selección de comentario arranca en 0"
    );

    feed(&mut app, &[key(KeyCode::Down)]);
    assert_eq!(
        app.selected_comment_index(),
        1,
        "`↓` baja la selección al segundo comentario"
    );

    // Clamp en el último.
    feed(&mut app, &[key(KeyCode::Down)]);
    assert_eq!(
        app.selected_comment_index(),
        1,
        "`↓` no debe pasar del último comentario"
    );

    feed(&mut app, &[key(KeyCode::Up)]);
    assert_eq!(
        app.selected_comment_index(),
        0,
        "`↑` sube la selección al primer comentario"
    );

    // Clamp en el primero.
    feed(&mut app, &[key(KeyCode::Up)]);
    assert_eq!(
        app.selected_comment_index(),
        0,
        "`↑` no debe pasar del primer comentario"
    );
}

/// TS-17: `↵` sobre el comentario seleccionado en la pantalla final SALTA a su
/// hilo: cierra la pantalla final (`mode()` deja de ser `Final`), el foco pasa a
/// `Focus::Thread`, y `selected_file_index`/`cursor_line` quedan en el archivo y
/// la línea del comentario seleccionado (segundo comentario: src/main.rs:20-22).
#[test]
fn ts17_enter_jumps_from_final_to_selected_comment_thread() {
    let mut app = app_with_two_comments_on_final();

    // Seleccionar el segundo comentario (rango en src/main.rs:20-22).
    feed(&mut app, &[key(KeyCode::Down)]);
    assert_eq!(app.selected_comment_index(), 1);

    feed(&mut app, &[key(KeyCode::Enter)]);

    assert_ne!(
        app.mode(),
        Mode::Final,
        "`↵` debe cerrar la pantalla final (mode ya no es Final)"
    );
    assert_eq!(
        app.focus(),
        Focus::Thread,
        "`↵` debe saltar al hilo del comentario seleccionado (foco Thread)"
    );
    assert_eq!(
        app.selected_file_index(),
        1,
        "el salto debe posicionar el archivo del comentario (src/main.rs = índice 1)"
    );
    // En src/main.rs las líneas aplanadas son: idx0=ctx19, idx1=added20,
    // idx2=added21, idx3=added22. El inicio del rango (línea 20) está en idx1.
    assert_eq!(
        app.cursor_line(),
        1,
        "el cursor debe quedar en la línea de inicio del comentario (línea 20 = índice 1)"
    );
}

/// TS-17: el salto al primer comentario (línea en src/diff.rs:15) posiciona el
/// archivo 0 y la línea 15 (índice aplanado 3).
#[test]
fn ts17_enter_jumps_to_first_comment_file_and_line() {
    let mut app = app_with_two_comments_on_final();

    assert_eq!(
        app.selected_comment_index(),
        0,
        "primer comentario seleccionado"
    );

    feed(&mut app, &[key(KeyCode::Enter)]);

    assert_eq!(app.focus(), Focus::Thread);
    assert_eq!(
        app.selected_file_index(),
        0,
        "el primer comentario está en src/diff.rs (índice 0)"
    );
    // En src/diff.rs: idx0=ctx12, idx1=ctx13, idx2=removed14, idx3=added15.
    assert_eq!(
        app.cursor_line(),
        3,
        "el cursor debe quedar en la línea 15 (índice aplanado 3)"
    );
}
