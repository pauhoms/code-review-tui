// Módulo PURO de resaltado de sintaxis (sin ratatui ni crossterm).
// Soporta PHP y TypeScript con un lexer de un solo paso por línea.

use std::path::Path;

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Php,
    TypeScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Str,
    Comment,
    Number,
    Plain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

// ---------------------------------------------------------------------------
// detect_language
// ---------------------------------------------------------------------------

pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "php" => Some(Language::Php),
        "ts" | "tsx" => Some(Language::TypeScript),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Keywords
// ---------------------------------------------------------------------------

const PHP_KEYWORDS: &[&str] = &[
    "abstract",
    "and",
    "array",
    "as",
    "break",
    "callable",
    "case",
    "catch",
    "class",
    "clone",
    "const",
    "continue",
    "declare",
    "default",
    "do",
    "echo",
    "else",
    "elseif",
    "empty",
    "enddeclare",
    "endfor",
    "endforeach",
    "endif",
    "endswitch",
    "endwhile",
    "enum",
    "extends",
    "final",
    "finally",
    "fn",
    "for",
    "foreach",
    "function",
    "global",
    "if",
    "implements",
    "include",
    "include_once",
    "instanceof",
    "insteadof",
    "interface",
    "isset",
    "list",
    "match",
    "namespace",
    "new",
    "or",
    "print",
    "private",
    "protected",
    "public",
    "readonly",
    "require",
    "require_once",
    "return",
    "static",
    "switch",
    "throw",
    "trait",
    "try",
    "unset",
    "use",
    "var",
    "while",
    "yield",
    "true",
    "false",
    "null",
];

const TS_KEYWORDS: &[&str] = &[
    "abstract",
    "any",
    "as",
    "async",
    "await",
    "boolean",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "declare",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "get",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "keyof",
    "let",
    "new",
    "null",
    "number",
    "of",
    "private",
    "protected",
    "public",
    "readonly",
    "return",
    "set",
    "static",
    "string",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "yield",
];

fn is_keyword(word: &str, lang: Language) -> bool {
    let keywords = match lang {
        Language::Php => PHP_KEYWORDS,
        Language::TypeScript => TS_KEYWORDS,
    };
    keywords.contains(&word)
}

// ---------------------------------------------------------------------------
// Helpers de carácter
// ---------------------------------------------------------------------------

/// Retorna true si el carácter puede ser parte de un identificador (incluye `_`).
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Retorna true si el carácter puede iniciar un identificador (no dígito).
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

// ---------------------------------------------------------------------------
// Lexer auxiliares
// ---------------------------------------------------------------------------

/// Consume un literal de cadena que comienza en `start` (byte offset).
/// El delimitador (`'`, `"` o `` ` ``) ya fue consumido y es `delim`.
/// Respeta `\"`, `\'`, `` \` `` para no cerrar antes de tiempo.
/// Devuelve el texto del token (incluye ambos delimitadores) y el nuevo índice.
fn lex_string(chars: &[(usize, char)], start: usize, delim: char) -> (String, usize) {
    let mut result = String::new();
    result.push(delim);
    let mut i = start;
    while i < chars.len() {
        let (_, c) = chars[i];
        i += 1;
        result.push(c);
        if c == '\\' && i < chars.len() {
            // Siguiente carácter es escapado: consumirlo sin verificar cierre.
            let (_, nc) = chars[i];
            result.push(nc);
            i += 1;
        } else if c == delim {
            break;
        }
    }
    (result, i)
}

/// Consume un comentario de línea (`//` o `#`). El marcador ya fue detectado
/// pero NO consumido del slice. Consume toda la parte restante.
fn lex_line_comment(chars: &[(usize, char)], start: usize) -> (String, usize) {
    let mut result = String::new();
    for &(_, c) in &chars[start..] {
        result.push(c);
    }
    (result, chars.len())
}

/// Consume un comentario de bloque `/* ... */` (TypeScript).
/// `start` apunta al `*` que sigue al `/` (el `/` ya fue detectado pero
/// no añadido). Retorna el texto completo (incluido `/*`) y el nuevo índice.
fn lex_block_comment(chars: &[(usize, char)], start: usize) -> (String, usize) {
    let mut result = "/*".to_string();
    // `start` apunta al `*`; lo incluimos y avanzamos.
    let mut i = start + 1; // saltamos el `*` ya añadido
    while i < chars.len() {
        let (_, c) = chars[i];
        result.push(c);
        i += 1;
        if c == '*' && i < chars.len() && chars[i].1 == '/' {
            result.push('/');
            i += 1;
            break;
        }
    }
    (result, i)
}

/// Consume un literal numérico (dígitos con punto decimal opcional).
fn lex_number(chars: &[(usize, char)], start: usize) -> (String, usize) {
    let mut result = String::new();
    let mut i = start;
    let mut has_dot = false;
    while i < chars.len() {
        let (_, c) = chars[i];
        if c.is_ascii_digit() {
            result.push(c);
            i += 1;
        } else if c == '.' && !has_dot && i + 1 < chars.len() && chars[i + 1].1.is_ascii_digit() {
            has_dot = true;
            result.push(c);
            i += 1;
        } else {
            break;
        }
    }
    (result, i)
}

/// Consume una palabra (identificador) que comienza con un carácter ident_start.
/// Detecta si es keyword o Plain.
fn lex_word(chars: &[(usize, char)], start: usize, lang: Language) -> (Token, usize) {
    let mut text = String::new();
    let mut i = start;
    while i < chars.len() && is_ident_char(chars[i].1) {
        text.push(chars[i].1);
        i += 1;
    }
    let kind = if is_keyword(&text, lang) {
        TokenKind::Keyword
    } else {
        TokenKind::Plain
    };
    (Token { kind, text }, i)
}

// ---------------------------------------------------------------------------
// tokenize
// ---------------------------------------------------------------------------

pub fn tokenize(line: &str, lang: Language) -> Vec<Token> {
    if line.is_empty() {
        return Vec::new();
    }

    // Recopilamos todos los (byte_offset, char) para indexar con seguridad.
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut tokens: Vec<Token> = Vec::new();
    // Acumulador de texto Plain.
    let mut plain_buf = String::new();
    let mut i = 0usize; // índice en `chars`

    // Función auxiliar para vaciar `plain_buf` como token Plain.
    macro_rules! flush_plain {
        () => {
            if !plain_buf.is_empty() {
                tokens.push(Token {
                    kind: TokenKind::Plain,
                    text: std::mem::take(&mut plain_buf),
                });
            }
        };
    }

    while i < chars.len() {
        let (_, c) = chars[i];

        // --- Comentario de línea: `//` ---
        if c == '/' && i + 1 < chars.len() && chars[i + 1].1 == '/' {
            flush_plain!();
            let (text, ni) = lex_line_comment(&chars, i);
            tokens.push(Token {
                kind: TokenKind::Comment,
                text,
            });
            i = ni;
            continue;
        }

        // --- Comentario de bloque: `/*` (solo TS) ---
        if lang == Language::TypeScript && c == '/' && i + 1 < chars.len() && chars[i + 1].1 == '*'
        {
            flush_plain!();
            // `start` para lex_block_comment apunta al `*` (índice i+1 en chars).
            let (text, ni) = lex_block_comment(&chars, i + 1);
            tokens.push(Token {
                kind: TokenKind::Comment,
                text,
            });
            i = ni;
            continue;
        }

        // --- Comentario con `#` (solo PHP) ---
        if lang == Language::Php && c == '#' {
            flush_plain!();
            let (text, ni) = lex_line_comment(&chars, i);
            tokens.push(Token {
                kind: TokenKind::Comment,
                text,
            });
            i = ni;
            continue;
        }

        // --- Literal de cadena: `'`, `"` ---
        if c == '\'' || c == '"' {
            flush_plain!();
            let (text, ni) = lex_string(&chars, i + 1, c);
            tokens.push(Token {
                kind: TokenKind::Str,
                text,
            });
            i = ni;
            continue;
        }

        // --- Template literal: `` ` `` (solo TS) ---
        if lang == Language::TypeScript && c == '`' {
            flush_plain!();
            let (text, ni) = lex_string(&chars, i + 1, '`');
            tokens.push(Token {
                kind: TokenKind::Str,
                text,
            });
            i = ni;
            continue;
        }

        // --- Número: dígito ---
        if c.is_ascii_digit() {
            flush_plain!();
            let (text, ni) = lex_number(&chars, i);
            tokens.push(Token {
                kind: TokenKind::Number,
                text,
            });
            i = ni;
            continue;
        }

        // --- Palabra: identificador (posible keyword) ---
        // NO aplica si es prefixado por `$` (PHP variable): `$x` → Plain
        if is_ident_start(c) {
            flush_plain!();
            let (tok, ni) = lex_word(&chars, i, lang);
            tokens.push(tok);
            i = ni;
            continue;
        }

        // --- Todo lo demás: acumular como Plain ---
        plain_buf.push(c);
        i += 1;
    }

    flush_plain!();
    tokens
}

// ---------------------------------------------------------------------------
// Tests unitarios
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn reconstruct(tokens: &[Token]) -> String {
        tokens.iter().map(|t| t.text.as_str()).collect()
    }

    fn has_token(tokens: &[Token], kind: TokenKind, text: &str) -> bool {
        tokens.iter().any(|t| t.kind == kind && t.text == text)
    }

    fn kind_of(tokens: &[Token], text: &str) -> Option<TokenKind> {
        tokens.iter().find(|t| t.text == text).map(|t| t.kind)
    }

    // --- detect_language ---

    #[test]
    fn detect_php_extension() {
        assert_eq!(detect_language(Path::new("a.php")), Some(Language::Php));
    }

    #[test]
    fn detect_ts_extension() {
        assert_eq!(
            detect_language(Path::new("b.ts")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn detect_tsx_extension() {
        assert_eq!(
            detect_language(Path::new("c.tsx")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn detect_rs_extension_is_none() {
        assert_eq!(detect_language(Path::new("d.rs")), None);
    }

    #[test]
    fn detect_no_extension_is_none() {
        assert_eq!(detect_language(Path::new("README")), None);
    }

    // --- Invariante de reconstrucción ---

    #[test]
    fn invariant_php_plain_line() {
        let line = "    foo->bar(baz);";
        assert_eq!(reconstruct(&tokenize(line, Language::Php)), line);
    }

    #[test]
    fn invariant_ts_mixed_line() {
        let line = "const a = \"hi\" + 7; // nota";
        assert_eq!(reconstruct(&tokenize(line, Language::TypeScript)), line);
    }

    #[test]
    fn invariant_php_with_variable() {
        let line = "    $x = 1;";
        assert_eq!(reconstruct(&tokenize(line, Language::Php)), line);
    }

    #[test]
    fn invariant_empty_line_php() {
        assert_eq!(tokenize("", Language::Php), Vec::<Token>::new());
    }

    #[test]
    fn invariant_empty_line_ts() {
        assert_eq!(tokenize("", Language::TypeScript), Vec::<Token>::new());
    }

    // --- Keyword vs substring ---

    #[test]
    fn return_value_is_not_keyword() {
        let tokens = tokenize("returnValue", Language::Php);
        assert_ne!(
            kind_of(&tokens, "returnValue"),
            Some(TokenKind::Keyword),
            "returnValue no debe ser Keyword"
        );
    }

    #[test]
    fn return_as_word_is_keyword_php() {
        let tokens = tokenize("return 1;", Language::Php);
        assert_eq!(kind_of(&tokens, "return"), Some(TokenKind::Keyword));
    }

    #[test]
    fn const_is_keyword_ts() {
        let tokens = tokenize("const x = 1;", Language::TypeScript);
        assert_eq!(kind_of(&tokens, "const"), Some(TokenKind::Keyword));
    }

    // --- Strings con comillas escapadas ---

    #[test]
    fn php_string_with_escaped_single_quote() {
        let line = r#"'it\'s'"#;
        let tokens = tokenize(line, Language::Php);
        assert_eq!(reconstruct(&tokens), line);
        // Debe haber exactamente un token Str que contenga toda la cadena.
        assert!(has_token(&tokens, TokenKind::Str, line));
    }

    #[test]
    fn ts_string_with_escaped_double_quote() {
        let line = r#""say \"hi\"""#;
        let tokens = tokenize(line, Language::TypeScript);
        assert_eq!(reconstruct(&tokens), line);
        assert!(has_token(&tokens, TokenKind::Str, line));
    }

    #[test]
    fn ts_template_literal_is_str() {
        let line = "const s = `hello`;";
        let tokens = tokenize(line, Language::TypeScript);
        assert_eq!(reconstruct(&tokens), line);
        assert!(has_token(&tokens, TokenKind::Str, "`hello`"));
    }

    // --- Comentarios ---

    #[test]
    fn php_hash_comment() {
        let line = "echo \"x\"; # fin";
        let tokens = tokenize(line, Language::Php);
        assert!(has_token(&tokens, TokenKind::Comment, "# fin"));
        assert_eq!(reconstruct(&tokens), line);
    }

    #[test]
    fn php_double_slash_comment() {
        let line = "return 'hola' . 42; // nota";
        let tokens = tokenize(line, Language::Php);
        assert!(has_token(&tokens, TokenKind::Comment, "// nota"));
        assert_eq!(reconstruct(&tokens), line);
    }

    #[test]
    fn ts_block_comment() {
        let line = "interface T { v = `hi`; } /* x */";
        let tokens = tokenize(line, Language::TypeScript);
        assert!(has_token(&tokens, TokenKind::Comment, "/* x */"));
        assert_eq!(reconstruct(&tokens), line);
    }

    #[test]
    fn ts_block_comment_unclosed() {
        // Si el comentario de bloque no cierra en la línea, toma hasta fin.
        let line = "foo /* sin cerrar";
        let tokens = tokenize(line, Language::TypeScript);
        assert!(has_token(&tokens, TokenKind::Comment, "/* sin cerrar"));
        assert_eq!(reconstruct(&tokens), line);
    }

    // --- Números decimales ---

    #[test]
    fn decimal_number_php() {
        let tokens = tokenize("x = 3.14;", Language::Php);
        assert_eq!(kind_of(&tokens, "3.14"), Some(TokenKind::Number));
    }

    #[test]
    fn decimal_number_ts() {
        let tokens = tokenize("n = 2.718;", Language::TypeScript);
        assert_eq!(kind_of(&tokens, "2.718"), Some(TokenKind::Number));
    }

    // --- Variable PHP `$x` es Plain ---

    #[test]
    fn php_variable_dollar_is_plain() {
        let tokens = tokenize("$myVar = 1;", Language::Php);
        // El `$myVar` o sus partes no son Keyword.
        for t in &tokens {
            assert_ne!(
                t.kind,
                TokenKind::Keyword,
                "ningún fragmento de $myVar debe ser Keyword; tok={t:?}"
            );
        }
        assert_eq!(reconstruct(&tokens), "$myVar = 1;");
    }

    #[test]
    fn php_dollar_alone_is_plain() {
        let tokens = tokenize("$ not_ident", Language::Php);
        assert_eq!(reconstruct(&tokens), "$ not_ident");
    }
}
