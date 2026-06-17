// Modelo puro de review y serialización Markdown. Sin I/O, sin TUI, sin git.

// --- Tipos públicos del modelo ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Old,
    New,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Lgtm,
    Ko,
    Undecided,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub file: String,
    pub side: Side,
    pub start_line: u32,
    pub end_line: u32,
    pub body: String,
}

// --- Modelo de review ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    comments: Vec<Comment>,
    general: Option<String>,
    verdict: Verdict,
}

impl Default for Review {
    fn default() -> Self {
        Self::new()
    }
}

impl Review {
    pub fn new() -> Review {
        Review {
            comments: Vec::new(),
            general: None,
            verdict: Verdict::Undecided,
        }
    }

    pub fn add_line_comment(&mut self, file: &str, side: Side, line: u32, body: &str) {
        self.comments.push(Comment {
            file: file.to_owned(),
            side,
            start_line: line,
            end_line: line,
            body: body.to_owned(),
        });
    }

    pub fn add_range_comment(&mut self, file: &str, side: Side, start: u32, end: u32, body: &str) {
        self.comments.push(Comment {
            file: file.to_owned(),
            side,
            start_line: start,
            end_line: end,
            body: body.to_owned(),
        });
    }

    pub fn set_general(&mut self, body: &str) {
        self.general = Some(body.to_owned());
    }

    pub fn set_verdict(&mut self, verdict: Verdict) {
        self.verdict = verdict;
    }

    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }
}

// --- Serialización Markdown ---

fn render_verdict(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Lgtm => "LGTM",
        Verdict::Ko => "KO",
        Verdict::Undecided => "sin decidir",
    }
}

fn comment_anchor(c: &Comment) -> String {
    if c.start_line == c.end_line {
        format!("{}:{}", c.file, c.start_line)
    } else {
        format!("{}:{}-{}", c.file, c.start_line, c.end_line)
    }
}

/// Serializa una `Review` a Markdown. Función pura: sin I/O, sin efectos.
pub fn to_markdown(review: &Review) -> String {
    let mut out = String::new();

    // Encabezado
    out.push_str("# Code review\n");
    out.push('\n');
    out.push_str(&format!(
        "**Veredicto:** {}\n",
        render_verdict(&review.verdict)
    ));

    // Comentario general (si existe)
    if let Some(general) = &review.general {
        out.push('\n');
        out.push_str("## Comentario general\n");
        out.push('\n');
        out.push_str(general);
        out.push('\n');
    }

    // Sección de comentarios de línea
    out.push('\n');
    out.push_str("## Comentarios\n");
    out.push('\n');

    if review.comments.is_empty() {
        out.push_str("Sin comentarios de línea.\n");
    } else {
        // Agrupamos por archivo manteniendo orden de primera aparición.
        let mut files_seen: Vec<&str> = Vec::new();
        for c in &review.comments {
            if !files_seen.contains(&c.file.as_str()) {
                files_seen.push(&c.file);
            }
        }

        let mut first_block = true;
        for file in files_seen {
            let file_comments: Vec<&Comment> =
                review.comments.iter().filter(|c| c.file == file).collect();

            for c in file_comments {
                if !first_block {
                    out.push('\n');
                }
                first_block = false;
                out.push_str(&format!("### {}\n", comment_anchor(c)));
                out.push('\n');
                out.push_str(&format!("> {}\n", c.body));
            }
        }
    }

    out
}

/// Devuelve el nombre de archivo de reporte dado la fecha inyectada (sin I/O).
pub fn report_filename(date: &str) -> String {
    format!("code-review-{date}.md")
}

// --- Tests unitarios ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- render_verdict ---

    #[test]
    fn render_verdict_lgtm_returns_lgtm() {
        assert_eq!(render_verdict(&Verdict::Lgtm), "LGTM");
    }

    #[test]
    fn render_verdict_ko_returns_ko() {
        assert_eq!(render_verdict(&Verdict::Ko), "KO");
    }

    #[test]
    fn render_verdict_undecided_returns_sin_decidir() {
        assert_eq!(render_verdict(&Verdict::Undecided), "sin decidir");
    }

    // --- comment_anchor ---

    #[test]
    fn comment_anchor_single_line_no_range_notation() {
        let c = Comment {
            file: "src/foo.rs".to_owned(),
            side: Side::New,
            start_line: 10,
            end_line: 10,
            body: "body".to_owned(),
        };
        assert_eq!(comment_anchor(&c), "src/foo.rs:10");
    }

    #[test]
    fn comment_anchor_range_uses_dash_notation() {
        let c = Comment {
            file: "src/foo.rs".to_owned(),
            side: Side::Old,
            start_line: 5,
            end_line: 8,
            body: "body".to_owned(),
        };
        assert_eq!(comment_anchor(&c), "src/foo.rs:5-8");
    }

    // --- add_line_comment: end_line == start_line ---

    #[test]
    fn add_line_comment_sets_end_line_equal_to_start_line() {
        let mut r = Review::new();
        r.add_line_comment("src/main.rs", Side::New, 42, "cuerpo");
        let c = &r.comments()[0];
        assert_eq!(c.start_line, 42);
        assert_eq!(c.end_line, 42);
    }

    // --- add_range_comment: preserva rango ---

    #[test]
    fn add_range_comment_preserves_start_and_end() {
        let mut r = Review::new();
        r.add_range_comment("src/lib.rs", Side::Old, 3, 7, "rango");
        let c = &r.comments()[0];
        assert_eq!(c.start_line, 3);
        assert_eq!(c.end_line, 7);
    }

    // --- orden de inserción ---

    #[test]
    fn comments_returns_in_insertion_order() {
        let mut r = Review::new();
        r.add_line_comment("a.rs", Side::New, 1, "primero");
        r.add_line_comment("b.rs", Side::New, 2, "segundo");
        r.add_line_comment("a.rs", Side::New, 3, "tercero");
        assert_eq!(r.comments()[0].body, "primero");
        assert_eq!(r.comments()[1].body, "segundo");
        assert_eq!(r.comments()[2].body, "tercero");
    }

    // --- veredicto por defecto ---

    #[test]
    fn new_review_has_undecided_verdict() {
        let r = Review::new();
        assert_eq!(r.verdict, Verdict::Undecided);
    }

    // --- Default trait coincide con new() ---

    #[test]
    fn default_review_equals_new_review() {
        let a = Review::new();
        let b = Review::default();
        assert_eq!(a, b);
    }

    // --- report_filename ---

    #[test]
    fn report_filename_formats_correctly() {
        assert_eq!(report_filename("2026-06-17"), "code-review-2026-06-17.md");
    }

    // --- to_markdown: formato sin comentarios ---

    #[test]
    fn to_markdown_no_line_comments_shows_sin_comentarios() {
        let mut r = Review::new();
        r.set_verdict(Verdict::Ko);
        r.set_general("general");
        let md = to_markdown(&r);
        assert!(md.contains("Sin comentarios de línea."));
    }

    // --- to_markdown: encabezado de anclaje línea única no usa rango ---

    #[test]
    fn to_markdown_single_line_anchor_no_dash_notation() {
        let mut r = Review::new();
        r.add_line_comment("f.rs", Side::New, 7, "cuerpo");
        let md = to_markdown(&r);
        assert!(md.contains("### f.rs:7"), "debe contener el anclaje");
        assert!(
            !md.contains("f.rs:7-7"),
            "no debe contener notación de rango"
        );
    }

    // --- to_markdown: cuerpo citado ---

    #[test]
    fn to_markdown_body_is_quoted_with_gt_prefix() {
        let mut r = Review::new();
        r.add_line_comment("x.rs", Side::New, 1, "texto del comentario");
        let md = to_markdown(&r);
        assert!(md.contains("> texto del comentario"));
    }

    // --- to_markdown: veredicto en encabezado ---

    #[test]
    fn to_markdown_verdict_appears_in_header() {
        let mut r = Review::new();
        r.set_verdict(Verdict::Lgtm);
        let md = to_markdown(&r);
        assert!(md.contains("**Veredicto:** LGTM"));
    }
}
