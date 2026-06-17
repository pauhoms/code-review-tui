use std::path::{Path, PathBuf};

// --- Error del crate ---

#[derive(Debug)]
pub enum DiffError {
    NoRepository(String),
    Git(git2::Error),
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffError::NoRepository(msg) => write!(f, "{msg}"),
            DiffError::Git(e) => write!(f, "error de git2: {e}"),
        }
    }
}

impl std::error::Error for DiffError {}

impl From<git2::Error> for DiffError {
    fn from(e: git2::Error) -> Self {
        DiffError::Git(e)
    }
}

// --- Modelo público ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Added,
    Removed,
    Context,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: PathBuf,
    pub status: FileStatus,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub files: Vec<FileDiff>,
}

// --- Adquisición ---

/// Descubre el repo git desde `path` hacia arriba y devuelve el diff completo
/// de cambios no commiteados (staged + unstaged + untracked + borrados).
pub fn collect(path: &Path) -> Result<Diff, DiffError> {
    let repo = git2::Repository::discover(path).map_err(|e| {
        // discover falla con GenericError si no hay repo; lo normalizamos.
        DiffError::NoRepository(format!(
            "no se encontró un repositorio git en '{path}' ni en ningún directorio padre: {e}",
            path = path.display()
        ))
    })?;

    let old_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);

    let git_diff = repo.diff_tree_to_workdir_with_index(old_tree.as_ref(), Some(&mut opts))?;

    build_diff(&git_diff)
}

fn build_diff(git_diff: &git2::Diff<'_>) -> Result<Diff, DiffError> {
    let mut files: Vec<FileDiff> = Vec::new();

    for (delta_idx, delta) in git_diff.deltas().enumerate() {
        let status = map_status(delta.status());
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .unwrap_or(Path::new(""))
            .to_path_buf();

        let patch = git2::Patch::from_diff(git_diff, delta_idx)?;
        let hunks = match patch {
            None => vec![],
            Some(mut p) => build_hunks(&mut p)?,
        };

        let additions = hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Added)
            .count();
        let deletions = hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Removed)
            .count();

        files.push(FileDiff {
            path,
            status,
            additions,
            deletions,
            hunks,
        });
    }

    Ok(Diff { files })
}

fn build_hunks(patch: &mut git2::Patch<'_>) -> Result<Vec<Hunk>, DiffError> {
    let num_hunks = patch.num_hunks();
    let mut hunks = Vec::with_capacity(num_hunks);

    for hunk_idx in 0..num_hunks {
        let (hunk_info, _) = patch.hunk(hunk_idx)?;
        let num_lines = patch.num_lines_in_hunk(hunk_idx)?;
        let mut lines = Vec::with_capacity(num_lines);

        for line_idx in 0..num_lines {
            let diff_line = patch.line_in_hunk(hunk_idx, line_idx)?;
            let kind = map_line_kind(diff_line.origin());
            // origin() puede devolver caracteres que no son líneas reales
            // (p. ej. 'F', 'H', 'B'). Solo procesamos los que nos interesan.
            let Some(kind) = kind else { continue };

            let content = strip_newline(diff_line.content());

            lines.push(Line {
                kind,
                old_lineno: diff_line.old_lineno(),
                new_lineno: diff_line.new_lineno(),
                content,
            });
        }

        hunks.push(Hunk {
            old_start: hunk_info.old_start(),
            old_lines: hunk_info.old_lines(),
            new_start: hunk_info.new_start(),
            new_lines: hunk_info.new_lines(),
            lines,
        });
    }

    Ok(hunks)
}

// --- Funciones de mapeo (pequeñas, testeables unitariamente) ---

fn map_status(s: git2::Delta) -> FileStatus {
    match s {
        git2::Delta::Added | git2::Delta::Untracked => FileStatus::Added,
        git2::Delta::Deleted => FileStatus::Deleted,
        git2::Delta::Renamed => FileStatus::Renamed,
        _ => FileStatus::Modified,
    }
}

fn map_line_kind(origin: char) -> Option<LineKind> {
    match origin {
        '+' => Some(LineKind::Added),
        '-' => Some(LineKind::Removed),
        ' ' => Some(LineKind::Context),
        _ => None,
    }
}

fn strip_newline(bytes: &[u8]) -> String {
    let s = std::str::from_utf8(bytes).unwrap_or("");
    s.trim_end_matches('\n').trim_end_matches('\r').to_owned()
}

// --- Tests unitarios ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- map_line_kind ---

    #[test]
    fn map_line_kind_plus_returns_added() {
        assert_eq!(map_line_kind('+'), Some(LineKind::Added));
    }

    #[test]
    fn map_line_kind_minus_returns_removed() {
        assert_eq!(map_line_kind('-'), Some(LineKind::Removed));
    }

    #[test]
    fn map_line_kind_space_returns_context() {
        assert_eq!(map_line_kind(' '), Some(LineKind::Context));
    }

    #[test]
    fn map_line_kind_other_returns_none() {
        for ch in ['F', 'H', 'B', 'O', 'S', 'E'] {
            assert_eq!(map_line_kind(ch), None, "carácter {ch} debería ser None");
        }
    }

    // --- map_status ---

    #[test]
    fn map_status_added_returns_added() {
        assert_eq!(map_status(git2::Delta::Added), FileStatus::Added);
    }

    #[test]
    fn map_status_untracked_returns_added() {
        assert_eq!(map_status(git2::Delta::Untracked), FileStatus::Added);
    }

    #[test]
    fn map_status_deleted_returns_deleted() {
        assert_eq!(map_status(git2::Delta::Deleted), FileStatus::Deleted);
    }

    #[test]
    fn map_status_renamed_returns_renamed() {
        assert_eq!(map_status(git2::Delta::Renamed), FileStatus::Renamed);
    }

    #[test]
    fn map_status_modified_returns_modified() {
        assert_eq!(map_status(git2::Delta::Modified), FileStatus::Modified);
    }

    // --- strip_newline ---

    #[test]
    fn strip_newline_removes_trailing_lf() {
        assert_eq!(strip_newline(b"hello\n"), "hello");
    }

    #[test]
    fn strip_newline_removes_trailing_crlf() {
        assert_eq!(strip_newline(b"hello\r\n"), "hello");
    }

    #[test]
    fn strip_newline_no_newline_unchanged() {
        assert_eq!(strip_newline(b"hello"), "hello");
    }

    #[test]
    fn strip_newline_empty_bytes() {
        assert_eq!(strip_newline(b""), "");
    }

    // --- DiffError display ---

    #[test]
    fn diff_error_no_repository_message_is_clear() {
        let msg = "no se encontró un repositorio git en '/tmp/x' ni en ningún directorio padre";
        let e = DiffError::NoRepository(msg.to_owned());
        let display = e.to_string();
        assert!(
            display.contains("no se encontró"),
            "el mensaje debería mencionar 'no se encontró', fue: {display}"
        );
    }

    // --- repo sin commits (HEAD inexistente) ---

    #[test]
    fn collect_on_repo_without_commits_returns_added_files() {
        use tempfile::tempdir;
        let dir = tempdir().expect("TempDir");
        let repo = git2::Repository::init(dir.path()).expect("git init");

        // Escribimos un archivo sin commitear nada (HEAD no existe).
        let file_path = dir.path().join("nuevo.txt");
        std::fs::write(&file_path, "hola\nmundo\n").expect("write");

        // Sin commits y sin stage, el archivo es untracked.
        let mut index = repo.index().expect("index");
        index
            .add_path(std::path::Path::new("nuevo.txt"))
            .expect("add");
        index.write().expect("write index");

        let model = collect(dir.path()).expect("collect sin commits debe funcionar");
        // Con HEAD inexistente todo aparece como añadido.
        assert!(
            !model.files.is_empty(),
            "debe haber al menos un archivo en el modelo"
        );
        assert!(
            model.files.iter().all(|f| f.status == FileStatus::Added),
            "todos los archivos deben ser Added cuando no hay commits"
        );
    }
}
