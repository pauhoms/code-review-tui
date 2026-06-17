//! Flujo completo de la TUI dirigido por eventos (fase 3): TS-10 (navegar →
//! comentar línea → comentario general → veredicto → finalizar escribe el .md
//! en disco), TS-11 (repo sin cambios: estado vacío y salida limpia) y TS-14
//! (pantalla final lista comentarios + general + veredicto).

mod common_tui;

use std::path::PathBuf;

use common_tui::{
    empty_diff, feed, feed_text, key, key_char, key_ctrl, render_to_string, sample_diff,
};
use crossterm::event::KeyCode;
use reviewv2::app::{App, Mode, Outcome};
use reviewv2::review::{Review, Side, Verdict};

/// TS-10: flujo completo dirigido por eventos que termina escribiendo en disco
/// un `.md` cuyo contenido coincide con el reporte esperado (mismos anclajes,
/// veredicto y comentario general).
#[test]
fn ts10_full_flow_writes_markdown_matching_expected_report() {
    let out_dir = tempfile::tempdir().expect("TempDir de salida");
    let date = "2026-06-17";

    // App con salida controlada por el test (directorio + fecha inyectados).
    let mut app = App::with_output(sample_diff(), out_dir.path().to_path_buf(), date.to_owned());

    // 1) Navegar: enfocar DIFF, ir a la línea añadida 15 de src/diff.rs.
    feed(&mut app, &[key_char('2')]);
    feed(&mut app, &[key_char('j'), key_char('j'), key_char('j')]);

    // 2) Comentar línea.
    feed(&mut app, &[key_char('c')]);
    feed_text(&mut app, "¿FileStatus cubre el caso renamed?");
    feed(&mut app, &[key_ctrl('s')]);
    assert_eq!(app.mode(), Mode::Navigate);

    // 3) Pantalla final con `g`.
    feed(&mut app, &[key_char('g')]);
    assert_eq!(app.mode(), Mode::Final, "`g` debe abrir la pantalla final");

    // 4) Comentario general: escribirlo en el campo general.
    feed_text(&mut app, "El refactor va bien pero falta cubrir renamed.");

    // 5) Elegir veredicto LGTM con las flechas.
    feed(&mut app, &[key(KeyCode::Left)]);

    // 6) Finalizar y guardar con Ctrl+S: devuelve Outcome::Saved(ruta).
    let outcome = app.handle_key(key_ctrl('s'));
    let written_path = match outcome {
        Outcome::Saved(p) => p,
        other => panic!("Ctrl+S en la pantalla final debe devolver Saved(ruta), fue: {other:?}"),
    };

    // La ruta escrita debe estar en el directorio inyectado y con el nombre de
    // reporte por fecha.
    let expected_path: PathBuf = out_dir.path().join("code-review-2026-06-17.md");
    assert_eq!(
        written_path, expected_path,
        "la ruta escrita debe respetar dir + fecha inyectados"
    );
    assert!(
        written_path.exists(),
        "el archivo Markdown debe existir en disco: {}",
        written_path.display()
    );

    // El contenido en disco coincide exactamente con el reporte esperado.
    let on_disk = std::fs::read_to_string(&written_path).expect("leer el .md escrito");

    let mut expected = Review::new();
    expected.add_line_comment(
        "src/diff.rs",
        Side::New,
        15,
        "¿FileStatus cubre el caso renamed?",
    );
    expected.set_general("El refactor va bien pero falta cubrir renamed.");
    expected.set_verdict(Verdict::Lgtm);
    let expected_md = reviewv2::review::to_markdown(&expected);

    assert_eq!(
        on_disk, expected_md,
        "el contenido escrito debe coincidir con to_markdown del Review esperado"
    );
}

/// TS-11: con un diff vacío (working tree limpio) la TUI muestra un estado vacío
/// informativo y `q` permite salir sin error.
#[test]
fn ts11_empty_diff_shows_informative_empty_state_and_quits() {
    let mut app = App::new(empty_diff());

    let screen = render_to_string(&app, 90, 12);
    assert!(
        screen.contains("No hay cambios sin commitear para revisar."),
        "debe mostrar el mensaje de estado vacío, pantalla:\n{screen}"
    );

    // `q` sale sin error: devuelve Outcome::Quit.
    let outcome = app.handle_key(key_char('q'));
    assert_eq!(
        outcome,
        Outcome::Quit,
        "`q` debe devolver Outcome::Quit en estado vacío"
    );
}

/// TS-11: `q` en la pantalla principal (con cambios) también devuelve Quit.
#[test]
fn ts11_q_quits_from_main_screen() {
    let mut app = App::new(sample_diff());
    let outcome = app.handle_key(key_char('q'));
    assert_eq!(
        outcome,
        Outcome::Quit,
        "`q` debe salir devolviendo Outcome::Quit"
    );
}

/// TS-14: la pantalla final lista en su parte superior todos los comentarios
/// agregados (cada uno con su anclaje `archivo:línea` / `archivo:Lini-Lfin`),
/// más el comentario general y el veredicto.
#[test]
fn ts14_final_screen_lists_comments_anchors_general_and_verdict() {
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

    // Abrir pantalla final y poner general + veredicto.
    feed(&mut app, &[key_char('g')]);
    assert_eq!(app.mode(), Mode::Final);
    feed_text(&mut app, "comentario general del cambio");
    feed(&mut app, &[key(KeyCode::Left)]); // veredicto LGTM

    let screen = render_to_string(&app, 120, 30);

    // Anclajes de ambos comentarios.
    assert!(
        screen.contains("src/diff.rs:15"),
        "la pantalla final debe listar el anclaje de línea `src/diff.rs:15`, pantalla:\n{screen}"
    );
    assert!(
        screen.contains("src/main.rs:20-22"),
        "la pantalla final debe listar el anclaje de rango `src/main.rs:20-22`, pantalla:\n{screen}"
    );

    // Comentario general visible.
    assert!(
        screen.contains("comentario general del cambio"),
        "la pantalla final debe mostrar el comentario general, pantalla:\n{screen}"
    );

    // Veredicto visible (LGTM seleccionado).
    assert!(
        screen.contains("LGTM"),
        "la pantalla final debe mostrar el veredicto LGTM, pantalla:\n{screen}"
    );
}
