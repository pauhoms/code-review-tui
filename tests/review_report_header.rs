//! TS-06: el reporte incluye en el encabezado el veredicto (LGTM o KO) y el
//! comentario general; un reporte con veredicto y general pero SIN comentarios
//! de línea se genera igualmente válido.

use reviewv2::review::{Review, Verdict};

/// TS-06: un reporte con veredicto KO y comentario general, sin comentarios de
/// línea, se serializa entero y de forma determinista. Fijamos el string exacto
/// para garantizar formato estable.
#[test]
fn ts06_report_with_verdict_and_general_without_line_comments() {
    let mut review = Review::new();
    review.set_verdict(Verdict::Ko);
    review.set_general("El refactor va bien, pero falta cubrir renamed.");

    let md = reviewv2::review::to_markdown(&review);

    let expected = "\
# Code review

**Veredicto:** KO

## Comentario general

El refactor va bien, pero falta cubrir renamed.

## Comentarios

Sin comentarios de línea.
";
    assert_eq!(
        md, expected,
        "el reporte sin comentarios de línea debe coincidir exactamente"
    );
}

/// TS-06: el encabezado refleja el veredicto LGTM cuando se fija así.
#[test]
fn ts06_header_shows_lgtm_verdict() {
    let mut review = Review::new();
    review.set_verdict(Verdict::Lgtm);
    review.set_general("Todo correcto.");

    let md = reviewv2::review::to_markdown(&review);

    assert!(
        md.contains("**Veredicto:** LGTM"),
        "el encabezado debe mostrar el veredicto LGTM, fue:\n{md}"
    );
    assert!(
        !md.contains("KO"),
        "un veredicto LGTM no debe contener el texto KO, fue:\n{md}"
    );
}

/// TS-06: sin veredicto decidido, el encabezado lo indica explícitamente
/// (estado "sin decidir") y el reporte sigue siendo válido.
#[test]
fn ts06_header_shows_undecided_verdict_by_default() {
    let mut review = Review::new();
    review.set_general("Pendiente de decisión.");

    let md = reviewv2::review::to_markdown(&review);

    assert!(
        md.contains("**Veredicto:** sin decidir"),
        "sin veredicto fijado el encabezado debe decir 'sin decidir', fue:\n{md}"
    );
}

/// TS-06 (apoyo): el nombre por defecto del reporte usa la fecha INYECTADA,
/// nunca `now()` interno.
#[test]
fn ts06_report_filename_uses_injected_date() {
    let name = reviewv2::review::report_filename("2026-06-17");
    assert_eq!(name, "code-review-2026-06-17.md");
}
