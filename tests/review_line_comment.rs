//! TS-04: comentario de una sola línea anclado a (archivo, lado, línea) y
//! renderizado en el reporte Markdown bajo `archivo:línea`.

use reviewv2::review::{Review, Side, Verdict};

/// TS-04: un comentario de una sola línea se ancla a (archivo, lado, número de
/// línea); el modelo conserva ese anclaje y el reporte Markdown muestra el
/// cuerpo del comentario bajo un encabezado `archivo:línea`.
#[test]
fn ts04_single_line_comment_is_anchored_and_rendered_under_file_line() {
    let mut review = Review::new();
    review.set_verdict(Verdict::Ko);
    review.set_general("Falta cubrir el caso renamed.");
    review.add_line_comment(
        "src/diff.rs",
        Side::New,
        15,
        "¿FileStatus cubre el caso renamed?",
    );

    // --- Estado del modelo ---
    assert_eq!(review.comments().len(), 1, "debe haber un comentario");
    let c = &review.comments()[0];
    assert_eq!(c.file, "src/diff.rs", "el archivo del anclaje");
    assert_eq!(c.side, Side::New, "el lado del anclaje");
    assert_eq!(c.start_line, 15, "la línea inicial del anclaje");
    assert_eq!(
        c.end_line, 15,
        "un comentario de una sola línea tiene end_line == start_line"
    );
    assert_eq!(c.body, "¿FileStatus cubre el caso renamed?");

    // --- Serialización Markdown ---
    let md = reviewv2::review::to_markdown(&review);

    // El encabezado de anclaje de una sola línea es exactamente `archivo:línea`.
    assert!(
        md.contains("### src/diff.rs:15"),
        "el reporte debe encabezar el comentario con `### src/diff.rs:15`, fue:\n{md}"
    );
    // El cuerpo del comentario aparece citado bajo ese encabezado.
    assert!(
        md.contains("> ¿FileStatus cubre el caso renamed?"),
        "el cuerpo del comentario debe citarse con `> `, fue:\n{md}"
    );
    // No debe aparecer una notación de rango para un comentario de una línea.
    assert!(
        !md.contains("src/diff.rs:15-15"),
        "un comentario de una sola línea NO debe serializarse como rango, fue:\n{md}"
    );
}
