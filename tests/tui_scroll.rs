//! La vista del diff debe seguir al cursor: al bajar más allá del alto visible
//! del panel, la línea del cursor no debe desaparecer (regresión).

mod common_tui;

use common_tui::{feed, key_char, render_to_string};
use reviewv2::app::App;
use reviewv2::diff::{Diff, FileDiff, FileStatus, Hunk, Line, LineKind};

/// Diff de un archivo con muchas líneas añadidas (más que el alto del panel).
fn tall_diff() -> Diff {
    let lines: Vec<Line> = (0..25)
        .map(|n| Line {
            kind: LineKind::Added,
            old_lineno: None,
            new_lineno: Some(n + 1),
            content: format!("L{n}"),
        })
        .collect();
    Diff {
        files: vec![FileDiff {
            path: "big.rs".into(),
            status: FileStatus::Modified,
            additions: 25,
            deletions: 0,
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 25,
                lines,
            }],
        }],
    }
}

/// Al bajar el cursor más allá del alto del panel, la línea del cursor sigue
/// visible (la vista hace scroll para seguirlo).
#[test]
fn cursor_stays_visible_when_scrolling_down_split() {
    let mut app = App::new(tall_diff());
    feed(&mut app, &[key_char('2')]); // foco DIFF
    for _ in 0..20 {
        feed(&mut app, &[key_char('j')]);
    }
    // El cursor está en el índice 20 (contenido "L20"), muy por debajo de un
    // panel de 12 filas de alto.
    let screen = render_to_string(&app, 80, 12);
    assert!(
        screen.contains("L20"),
        "la línea bajo el cursor debe seguir visible al bajar (split), pantalla:\n{screen}"
    );
}

/// Lo mismo en la vista unificada.
#[test]
fn cursor_stays_visible_when_scrolling_down_unified() {
    let mut app = App::new(tall_diff());
    feed(&mut app, &[key_char('2'), key_char('t')]); // foco DIFF + unificado
    for _ in 0..20 {
        feed(&mut app, &[key_char('j')]);
    }
    let screen = render_to_string(&app, 80, 12);
    assert!(
        screen.contains("L20"),
        "la línea bajo el cursor debe seguir visible al bajar (unificado), pantalla:\n{screen}"
    );
}
