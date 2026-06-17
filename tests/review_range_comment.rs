//! TS-05: comentario multilínea anclado a un rango (archivo, lado, inicio, fin)
//! y serializado como `archivo:Lini-Lfin`.

use reviewv2::review::{Review, Side, Verdict};

/// TS-05: un comentario multilínea se ancla a un rango (archivo, lado, línea
/// inicio, línea fin); el modelo conserva el rango y el reporte lo serializa
/// indicando `archivo:Lini-Lfin`.
#[test]
fn ts05_range_comment_is_anchored_and_rendered_as_range() {
    let mut review = Review::new();
    review.set_verdict(Verdict::Ko);
    review.add_range_comment(
        "src/diff.rs",
        Side::New,
        20,
        22,
        "Extraer este loop a una función con test propio.",
    );

    // --- Estado del modelo ---
    assert_eq!(review.comments().len(), 1, "debe haber un comentario");
    let c = &review.comments()[0];
    assert_eq!(c.file, "src/diff.rs");
    assert_eq!(c.side, Side::New);
    assert_eq!(c.start_line, 20, "línea de inicio del rango");
    assert_eq!(c.end_line, 22, "línea de fin del rango");

    // --- Serialización Markdown ---
    let md = reviewv2::review::to_markdown(&review);

    assert!(
        md.contains("### src/diff.rs:20-22"),
        "el reporte debe encabezar el comentario de rango con `### src/diff.rs:20-22`, fue:\n{md}"
    );
    assert!(
        md.contains("> Extraer este loop a una función con test propio."),
        "el cuerpo del comentario de rango debe citarse con `> `, fue:\n{md}"
    );
}
