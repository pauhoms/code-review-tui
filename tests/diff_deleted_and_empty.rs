//! TS-03: archivo eliminado, y repo sin cambios -> modelo vacío.

mod common;

use common::{commit_all, init_repo, remove_file, stage, write_file};
use reviewv2::diff::{self, FileStatus, LineKind};

/// TS-03 (parte A): Un archivo eliminado aparece en el modelo como Deleted, con
/// todas sus líneas marcadas como eliminadas (sin número de línea nuevo).
#[test]
fn ts03_deleted_file_appears_as_deleted_with_all_lines_removed() {
    let (dir, repo) = init_repo();
    let root = dir.path();

    // Estado base: archivo versionado que luego borraremos.
    write_file(root, "doomed.txt", "a\nb\nc\n");
    stage(&repo, "doomed.txt");
    commit_all(&repo, "base");

    // Borrar el archivo del workdir.
    remove_file(root, "doomed.txt");

    let model = diff::collect(root).expect("collect debe tener éxito");

    let deleted = model
        .files
        .iter()
        .find(|f| f.path.ends_with("doomed.txt"))
        .expect("el archivo borrado debe estar en el modelo");

    assert_eq!(deleted.status, FileStatus::Deleted);
    assert_eq!(deleted.additions, 0, "ninguna adición");
    assert_eq!(deleted.deletions, 3, "tres líneas eliminadas");

    let all_lines: Vec<_> = deleted.hunks.iter().flat_map(|h| &h.lines).collect();
    assert_eq!(all_lines.len(), 3, "tres líneas en total");
    assert!(
        all_lines.iter().all(|l| l.kind == LineKind::Removed),
        "todas las líneas deben ser Removed"
    );

    let contents: Vec<&str> = all_lines.iter().map(|l| l.content.as_str()).collect();
    assert_eq!(contents, vec!["a", "b", "c"]);

    // Eliminadas: tienen old_lineno y no new_lineno.
    assert_eq!(all_lines[0].old_lineno, Some(1));
    assert_eq!(all_lines[0].new_lineno, None);
    assert_eq!(all_lines[1].old_lineno, Some(2));
    assert_eq!(all_lines[2].old_lineno, Some(3));
}

/// TS-03 (parte B): Un repo sin cambios no commiteados produce un modelo vacío
/// (lista de archivos vacía), sin error.
#[test]
fn ts03_clean_repo_yields_empty_model() {
    let (dir, repo) = init_repo();
    let root = dir.path();

    write_file(root, "tracked.txt", "stable\ncontent\n");
    stage(&repo, "tracked.txt");
    commit_all(&repo, "base");

    // No hay cambios posteriores: working tree limpio respecto a HEAD.
    let model = diff::collect(root).expect("collect debe tener éxito");

    assert!(
        model.files.is_empty(),
        "un repo limpio debe producir una lista de archivos vacía, fue {:?}",
        model.files
    );
}
