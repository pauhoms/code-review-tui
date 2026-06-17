//! Tests headless de render de la TUI (fase 3) con `ratatui::TestBackend`.
//! Cubre TS-07 (lista de archivos + diff con números y +/−), TS-12 (split por
//! defecto, columnas OLD/NEW, toggle `t` a unificado) y TS-13 (paneles
//! numerados `[1]`/`[2]` y foco conmutable con `1`/`2`).

mod common_tui;

use common_tui::{any_row_contains, feed, key_char, render_to_rows, render_to_string, sample_diff};
use reviewv2::app::{App, Focus, ViewMode};

/// TS-07: la TUI renderiza el panel de lista de archivos y el diff del archivo
/// seleccionado, con números de línea y marcas `+`/`−`.
#[test]
fn ts07_renders_file_list_and_selected_diff_with_line_numbers_and_marks() {
    let app = App::new(sample_diff());
    let rows = render_to_rows(&app, 120, 30);
    let screen = rows.join("\n");

    // Lista de archivos: ambos archivos del diff aparecen.
    assert!(
        any_row_contains(&rows, "diff.rs"),
        "la lista de archivos debe mostrar `diff.rs`, pantalla:\n{screen}"
    );
    assert!(
        any_row_contains(&rows, "main.rs"),
        "la lista de archivos debe mostrar `main.rs`, pantalla:\n{screen}"
    );

    // Diff del archivo seleccionado (el primero, src/diff.rs): contenido real.
    assert!(
        screen.contains("pub struct FileDiff {"),
        "debe renderizar el contenido del diff del archivo seleccionado, pantalla:\n{screen}"
    );

    // Marcas +/− del diff (línea eliminada `u8`, línea añadida `FileStatus`).
    assert!(
        screen.contains('-') || screen.contains('−'),
        "debe haber una marca de eliminación, pantalla:\n{screen}"
    );
    assert!(
        screen.contains('+'),
        "debe haber una marca de adición `+`, pantalla:\n{screen}"
    );

    // Números de línea del diff (la añadida es la 15 en lado nuevo, contexto 12).
    assert!(
        screen.contains("15"),
        "debe mostrar el número de línea 15, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("12"),
        "debe mostrar el número de línea 12, pantalla:\n{screen}"
    );
}

/// TS-12 (parte split): por defecto el modo de vista es Split y se renderizan
/// columnas OLD y NEW; la eliminada aparece en la columna OLD, la añadida en la
/// NEW, y el contexto en ambas.
#[test]
fn ts12_starts_in_split_mode_with_old_and_new_columns() {
    let app = App::new(sample_diff());

    assert_eq!(
        app.view_mode(),
        ViewMode::Split,
        "el modo de vista debe arrancar en Split"
    );

    let rows = render_to_rows(&app, 120, 30);
    let screen = rows.join("\n");

    // Encabezados de columnas OLD / NEW.
    assert!(
        screen.contains("OLD"),
        "el split debe rotular la columna OLD, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("NEW"),
        "el split debe rotular la columna NEW, pantalla:\n{screen}"
    );

    // La línea eliminada (`pub status: u8,`) debe estar a la IZQUIERDA de la
    // línea añadida (`pub status: FileStatus,`) en la fila que las contiene, o
    // en filas separadas donde la eliminada solo aparezca en la mitad izquierda.
    // Verificación robusta: hay alguna fila que contiene el texto viejo y otra
    // (o la misma) con el texto nuevo, y el texto nuevo aparece en una columna
    // a la derecha del ancho/2 en su fila.
    let half = 120 / 2;
    let new_on_right = rows.iter().any(|r| {
        if let Some(idx) = r.find("FileStatus,") {
            idx as u16 >= half
        } else {
            false
        }
    });
    assert!(
        new_on_right,
        "la línea añadida debe renderizarse en la columna NEW (mitad derecha), pantalla:\n{screen}"
    );

    let old_on_left = rows.iter().any(|r| {
        if let Some(idx) = r.find("pub status: u8,") {
            (idx as u16) < half
        } else {
            false
        }
    });
    assert!(
        old_on_left,
        "la línea eliminada debe renderizarse en la columna OLD (mitad izquierda), pantalla:\n{screen}"
    );
}

/// TS-12 (parte toggle): la tecla `t` alterna de split a unificado, lo que es
/// observable en el estado y en el render (desaparece la rotulación OLD/NEW de
/// columnas y la añadida deja de estar confinada a la mitad derecha).
#[test]
fn ts12_t_toggles_to_unified_view() {
    let mut app = App::new(sample_diff());
    assert_eq!(app.view_mode(), ViewMode::Split);

    feed(&mut app, &[key_char('t')]);
    assert_eq!(
        app.view_mode(),
        ViewMode::Unified,
        "la tecla `t` debe alternar a la vista unificada"
    );

    let rows = render_to_rows(&app, 120, 30);
    let screen = rows.join("\n");

    // En unificado la línea añadida ya NO está confinada a la mitad derecha:
    // aparece en una sola columna. Su prefijo `+` está cerca del inicio.
    let half = 120 / 2;
    let added_appears_left_half = rows.iter().any(|r| {
        if let Some(idx) = r.find("pub status: FileStatus,") {
            (idx as u16) < half
        } else {
            false
        }
    });
    assert!(
        added_appears_left_half,
        "en unificado la añadida debe poder aparecer en la mitad izquierda (columna única), pantalla:\n{screen}"
    );

    // Toggle de vuelta con `t` restaura split.
    feed(&mut app, &[key_char('t')]);
    assert_eq!(
        app.view_mode(),
        ViewMode::Split,
        "`t` de nuevo debe volver a Split"
    );
}

/// TS-13: paneles numerados `[1]` FILES y `[2]` DIFF; el foco arranca en FILES,
/// `2` lo lleva a DIFF y `1` de vuelta a FILES.
#[test]
fn ts13_panels_are_numbered_and_focus_switches_with_digits() {
    let mut app = App::new(sample_diff());

    // Marcadores de panel numerados presentes en el render.
    let screen = render_to_string(&app, 120, 30);
    assert!(
        screen.contains("[1]"),
        "el panel FILES debe mostrarse numerado `[1]`, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("[2]"),
        "el panel DIFF debe mostrarse numerado `[2]`, pantalla:\n{screen}"
    );

    // Foco inicial en FILES.
    assert_eq!(
        app.focus(),
        Focus::Files,
        "el foco debe arrancar en el panel FILES"
    );

    // `2` enfoca DIFF.
    feed(&mut app, &[key_char('2')]);
    assert_eq!(
        app.focus(),
        Focus::Diff,
        "la tecla `2` debe enfocar el panel DIFF"
    );

    // `1` vuelve a FILES.
    feed(&mut app, &[key_char('1')]);
    assert_eq!(
        app.focus(),
        Focus::Files,
        "la tecla `1` debe enfocar el panel FILES"
    );
}

/// TS-13: el destino de las teclas de navegación depende del panel enfocado.
/// Con FILES enfocado, `j` cambia el archivo seleccionado; con DIFF enfocado,
/// `j` mueve el cursor de línea (y NO cambia el archivo seleccionado).
#[test]
fn ts13_navigation_keys_target_the_focused_panel() {
    let mut app = App::new(sample_diff());

    // Foco FILES: `j` avanza el archivo seleccionado de 0 a 1.
    assert_eq!(app.focus(), Focus::Files);
    let file_before = app.selected_file_index();
    feed(&mut app, &[key_char('j')]);
    assert_eq!(
        app.selected_file_index(),
        file_before + 1,
        "con FILES enfocado, `j` debe avanzar el archivo seleccionado"
    );

    // Volvemos al primer archivo y enfocamos DIFF.
    feed(&mut app, &[key_char('k')]);
    assert_eq!(app.selected_file_index(), 0);
    feed(&mut app, &[key_char('2')]);
    assert_eq!(app.focus(), Focus::Diff);

    // Foco DIFF: `j` mueve el cursor de línea y NO cambia el archivo.
    let file_idx = app.selected_file_index();
    let line_before = app.cursor_line();
    feed(&mut app, &[key_char('j')]);
    assert_eq!(
        app.selected_file_index(),
        file_idx,
        "con DIFF enfocado, `j` NO debe cambiar el archivo seleccionado"
    );
    assert_eq!(
        app.cursor_line(),
        line_before + 1,
        "con DIFF enfocado, `j` debe avanzar el cursor de línea"
    );
}
