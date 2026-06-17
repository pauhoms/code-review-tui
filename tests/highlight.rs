//! Tests de aceptación EN ROJO para el módulo PURO `reviewv2::highlight`
//! (resaltado de sintaxis liviano para PHP y TypeScript).
//!
//! Contrato bajo prueba (sin dependencias de ratatui/crossterm):
//!
//! ```ignore
//! pub enum Language { Php, TypeScript }
//! pub enum TokenKind { Keyword, Str, Comment, Number, Plain }
//! pub struct Token { pub kind: TokenKind, pub text: String }
//! pub fn detect_language(path: &std::path::Path) -> Option<Language>;
//! pub fn tokenize(line: &str, lang: Language) -> Vec<Token>;
//! ```
//!
//! INVARIANTE DURO (sagrado): la concatenación de `token.text` en orden es
//! EXACTAMENTE igual a la línea de entrada — sin perder ni agregar caracteres
//! (incluidos espacios).

use std::path::Path;

use reviewv2::highlight::{Language, Token, TokenKind, detect_language, tokenize};

// --- Helpers de test (solo bajo tests/) ---

/// Reconstruye la línea concatenando el texto de los tokens en orden.
fn reconstruct(tokens: &[Token]) -> String {
    let mut s = String::new();
    for t in tokens {
        s.push_str(&t.text);
    }
    s
}

/// ¿Existe un token cuyo `text` sea EXACTAMENTE `text` y cuyo `kind` sea `kind`?
fn has_token(tokens: &[Token], kind: TokenKind, text: &str) -> bool {
    tokens.iter().any(|t| t.kind == kind && t.text == text)
}

/// `TokenKind` del primer token cuyo `text` sea exactamente `text`.
fn kind_of(tokens: &[Token], text: &str) -> Option<TokenKind> {
    tokens.iter().find(|t| t.text == text).map(|t| t.kind)
}

// ---------------------------------------------------------------------------
// HL-01: detect_language por extensión.
// ---------------------------------------------------------------------------

#[test]
fn hl01_detect_language_php_extension_is_php() {
    assert_eq!(
        detect_language(Path::new("src/ejemplo.php")),
        Some(Language::Php),
        "`.php` debe detectarse como Php"
    );
}

#[test]
fn hl01_detect_language_ts_extension_is_typescript() {
    assert_eq!(
        detect_language(Path::new("src/index.ts")),
        Some(Language::TypeScript),
        "`.ts` debe detectarse como TypeScript"
    );
}

#[test]
fn hl01_detect_language_tsx_extension_is_typescript() {
    assert_eq!(
        detect_language(Path::new("src/App.tsx")),
        Some(Language::TypeScript),
        "`.tsx` debe detectarse como TypeScript"
    );
}

#[test]
fn hl01_detect_language_rust_extension_is_none() {
    assert_eq!(
        detect_language(Path::new("src/main.rs")),
        None,
        "`.rs` no es soportado, debe ser None"
    );
}

#[test]
fn hl01_detect_language_no_extension_is_none() {
    assert_eq!(
        detect_language(Path::new("README")),
        None,
        "un archivo sin extensión debe ser None"
    );
}

#[test]
fn hl01_detect_language_unknown_extension_is_none() {
    assert_eq!(
        detect_language(Path::new("notes.md")),
        None,
        "una extensión desconocida (`.md`) debe ser None"
    );
}

// ---------------------------------------------------------------------------
// HL-02: tokenize PHP — keyword, string, comentario y número.
// ---------------------------------------------------------------------------

#[test]
fn hl02_php_keyword_string_comment_and_number_have_expected_kinds() {
    // Línea PHP con: keyword `return`, string `'hola'`, número `42` y
    // comentario `// nota`.
    let line = "return 'hola' . 42; // nota";
    let tokens = tokenize(line, Language::Php);

    assert_eq!(
        kind_of(&tokens, "return"),
        Some(TokenKind::Keyword),
        "`return` debe ser Keyword en PHP; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Str, "'hola'"),
        "el literal `'hola'` debe ser Str (comillas incluidas); tokens: {tokens:?}"
    );
    assert_eq!(
        kind_of(&tokens, "42"),
        Some(TokenKind::Number),
        "`42` debe ser Number en PHP; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Comment, "// nota"),
        "el comentario `// nota` debe ser Comment (incluyendo `//`); tokens: {tokens:?}"
    );
}

#[test]
fn hl02_php_reconstruction_is_exact() {
    let line = "return 'hola' . 42; // nota";
    let tokens = tokenize(line, Language::Php);
    assert_eq!(
        reconstruct(&tokens),
        line,
        "la concatenación de los tokens debe reconstruir la línea PHP EXACTA"
    );
}

#[test]
fn hl02_php_hash_comment_is_comment_and_double_quoted_string_is_str() {
    // PHP también admite comentarios con `#` y strings con comillas dobles.
    let line = "echo \"x\"; # fin";
    let tokens = tokenize(line, Language::Php);

    assert_eq!(
        kind_of(&tokens, "echo"),
        Some(TokenKind::Keyword),
        "`echo` debe ser Keyword en PHP; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Str, "\"x\""),
        "el literal `\"x\"` debe ser Str; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Comment, "# fin"),
        "el comentario `# fin` debe ser Comment; tokens: {tokens:?}"
    );
    assert_eq!(
        reconstruct(&tokens),
        line,
        "la reconstrucción debe ser exacta también con comentario `#`"
    );
}

#[test]
fn hl02_php_variable_is_not_keyword_and_reconstructs() {
    // La variable PHP `$x` es Plain (no keyword), y el texto se preserva.
    let line = "    $x = 1;";
    let tokens = tokenize(line, Language::Php);

    assert_ne!(
        kind_of(&tokens, "$x"),
        Some(TokenKind::Keyword),
        "una variable `$x` NO debe clasificarse como Keyword; tokens: {tokens:?}"
    );
    assert_eq!(
        kind_of(&tokens, "1"),
        Some(TokenKind::Number),
        "`1` debe ser Number; tokens: {tokens:?}"
    );
    assert_eq!(
        reconstruct(&tokens),
        line,
        "la reconstrucción debe preservar la indentación y la variable PHP"
    );
}

// ---------------------------------------------------------------------------
// HL-03: tokenize TypeScript — keyword, string/template, comentario y número.
// ---------------------------------------------------------------------------

#[test]
fn hl03_typescript_keyword_string_comment_and_number_have_expected_kinds() {
    // Línea TS con: keyword `const`, string `"hi"`, número `7` y comentario
    // `// nota`.
    let line = "const a = \"hi\" + 7; // nota";
    let tokens = tokenize(line, Language::TypeScript);

    assert_eq!(
        kind_of(&tokens, "const"),
        Some(TokenKind::Keyword),
        "`const` debe ser Keyword en TS; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Str, "\"hi\""),
        "el literal `\"hi\"` debe ser Str; tokens: {tokens:?}"
    );
    assert_eq!(
        kind_of(&tokens, "7"),
        Some(TokenKind::Number),
        "`7` debe ser Number en TS; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Comment, "// nota"),
        "el comentario `// nota` debe ser Comment; tokens: {tokens:?}"
    );
}

#[test]
fn hl03_typescript_reconstruction_is_exact() {
    let line = "const a = \"hi\" + 7; // nota";
    let tokens = tokenize(line, Language::TypeScript);
    assert_eq!(
        reconstruct(&tokens),
        line,
        "la concatenación de los tokens debe reconstruir la línea TS EXACTA"
    );
}

#[test]
fn hl03_typescript_template_literal_is_str_and_block_comment_is_comment() {
    // TS: template literal `` `hi` `` como Str, comentario de bloque `/* x */`,
    // keyword `interface`.
    let line = "interface T { v = `hi`; } /* x */";
    let tokens = tokenize(line, Language::TypeScript);

    assert_eq!(
        kind_of(&tokens, "interface"),
        Some(TokenKind::Keyword),
        "`interface` debe ser Keyword en TS; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Str, "`hi`"),
        "el template literal `` `hi` `` debe ser Str; tokens: {tokens:?}"
    );
    assert!(
        has_token(&tokens, TokenKind::Comment, "/* x */"),
        "el comentario de bloque `/* x */` debe ser Comment; tokens: {tokens:?}"
    );
    assert_eq!(
        reconstruct(&tokens),
        line,
        "la reconstrucción debe ser exacta con template y comentario de bloque"
    );
}

// ---------------------------------------------------------------------------
// HL-04: invariante de no-pérdida en casos sin nada especial + línea vacía.
// ---------------------------------------------------------------------------

#[test]
fn hl04_php_plain_line_never_loses_text() {
    // Línea sin keywords/strings/comentarios/números reconocidos.
    let line = "    foo->bar(baz);";
    let tokens = tokenize(line, Language::Php);
    assert_eq!(
        reconstruct(&tokens),
        line,
        "una línea PHP sin tokens especiales nunca debe perder texto"
    );
}

#[test]
fn hl04_typescript_plain_line_never_loses_text() {
    let line = "  obj.method(arg);";
    let tokens = tokenize(line, Language::TypeScript);
    assert_eq!(
        reconstruct(&tokens),
        line,
        "una línea TS sin tokens especiales nunca debe perder texto"
    );
}

#[test]
fn hl04_unsupported_path_is_none_so_caller_does_not_highlight() {
    // Para una ruta no soportada el llamador no debe resaltar: detect → None.
    assert_eq!(
        detect_language(Path::new("data/config.yaml")),
        None,
        "una ruta no soportada debe devolver None para que el llamador no resalte"
    );
}

#[test]
fn hl04_empty_line_php_is_empty_vec() {
    // Decisión determinista fijada por el contrato: la línea vacía produce
    // `vec![]` (cero tokens), NO un token vacío.
    let tokens = tokenize("", Language::Php);
    assert_eq!(
        tokens,
        Vec::<Token>::new(),
        "la línea vacía debe producir `vec![]` (cero tokens) en PHP"
    );
}

#[test]
fn hl04_empty_line_typescript_is_empty_vec() {
    let tokens = tokenize("", Language::TypeScript);
    assert_eq!(
        tokens,
        Vec::<Token>::new(),
        "la línea vacía debe producir `vec![]` (cero tokens) en TS"
    );
}
