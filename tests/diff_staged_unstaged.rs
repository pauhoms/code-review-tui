//! TS-01: cambios staged + unstaged sobre archivos ya versionados.

mod common;

use common::{commit_all, init_repo, stage, write_file};
use reviewv2::diff::{self, FileStatus, LineKind};

/// TS-01: En un repo con cambios staged y unstaged sobre archivos ya
/// versionados, el modelo lista esos archivos como Modificados, cada uno con
/// sus hunks y líneas tipadas, con números de línea viejo/nuevo correctos.
#[test]
fn ts01_lists_modified_files_with_typed_lines_and_line_numbers() {
    let (dir, repo) = init_repo();
    let root = dir.path();

    // Estado base versionado: dos archivos commiteados.
    write_file(root, "staged.txt", "line1\nline2\nline3\n");
    write_file(root, "unstaged.txt", "alpha\nbeta\ngamma\n");
    stage(&repo, "staged.txt");
    stage(&repo, "unstaged.txt");
    commit_all(&repo, "base");

    // Cambio STAGED en staged.txt: reemplaza "line2" por "line2-edited".
    write_file(root, "staged.txt", "line1\nline2-edited\nline3\n");
    stage(&repo, "staged.txt");

    // Cambio UNSTAGED en unstaged.txt: reemplaza "beta" por "beta-edited"
    // (escrito en workdir, NO añadido al index).
    write_file(root, "unstaged.txt", "alpha\nbeta-edited\ngamma\n");

    let model = diff::collect(root).expect("collect debe tener éxito");

    // Ambos archivos modificados deben aparecer.
    assert_eq!(
        model.files.len(),
        2,
        "deben listarse exactamente 2 archivos modificados"
    );

    let staged = model
        .files
        .iter()
        .find(|f| f.path.ends_with("staged.txt"))
        .expect("staged.txt debe estar en el modelo");
    let unstaged = model
        .files
        .iter()
        .find(|f| f.path.ends_with("unstaged.txt"))
        .expect("unstaged.txt debe estar en el modelo");

    // Ambos son modificaciones de archivos versionados.
    assert_eq!(staged.status, FileStatus::Modified);
    assert_eq!(unstaged.status, FileStatus::Modified);

    // Contadores +/−: una línea cambiada = 1 añadida + 1 eliminada.
    assert_eq!(staged.additions, 1, "staged.txt: 1 línea añadida");
    assert_eq!(staged.deletions, 1, "staged.txt: 1 línea eliminada");
    assert_eq!(unstaged.additions, 1, "unstaged.txt: 1 línea añadida");
    assert_eq!(unstaged.deletions, 1, "unstaged.txt: 1 línea eliminada");

    // El archivo staged tiene al menos un hunk.
    assert_eq!(staged.hunks.len(), 1, "staged.txt: un único hunk");
    let hunk = &staged.hunks[0];

    // Verificamos líneas tipadas con sus números viejo/nuevo.
    // Esperado para staged.txt:
    //   context  "line1"        old=1 new=1
    //   removed  "line2"        old=2 new=None
    //   added    "line2-edited" old=None new=2
    //   context  "line3"        old=3 new=3
    let removed: Vec<_> = hunk
        .lines
        .iter()
        .filter(|l| l.kind == LineKind::Removed)
        .collect();
    let added: Vec<_> = hunk
        .lines
        .iter()
        .filter(|l| l.kind == LineKind::Added)
        .collect();
    let context: Vec<_> = hunk
        .lines
        .iter()
        .filter(|l| l.kind == LineKind::Context)
        .collect();

    assert_eq!(removed.len(), 1, "una línea eliminada");
    assert_eq!(added.len(), 1, "una línea añadida");
    assert_eq!(context.len(), 2, "dos líneas de contexto");

    // Línea eliminada: tiene número viejo, no nuevo.
    assert_eq!(removed[0].content, "line2");
    assert_eq!(removed[0].old_lineno, Some(2));
    assert_eq!(removed[0].new_lineno, None);

    // Línea añadida: tiene número nuevo, no viejo.
    assert_eq!(added[0].content, "line2-edited");
    assert_eq!(added[0].old_lineno, None);
    assert_eq!(added[0].new_lineno, Some(2));

    // Contexto: ambos números presentes y correctos.
    let ctx_line1 = context
        .iter()
        .find(|l| l.content == "line1")
        .expect("contexto line1");
    assert_eq!(ctx_line1.old_lineno, Some(1));
    assert_eq!(ctx_line1.new_lineno, Some(1));

    let ctx_line3 = context
        .iter()
        .find(|l| l.content == "line3")
        .expect("contexto line3");
    assert_eq!(ctx_line3.old_lineno, Some(3));
    assert_eq!(ctx_line3.new_lineno, Some(3));
}
