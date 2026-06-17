//! TS-02: archivo nuevo no rastreado (untracked).

mod common;

use common::{commit_all, init_repo, stage, write_file};
use reviewv2::diff::{self, FileStatus, LineKind};

/// TS-02: Un archivo nuevo no rastreado aparece en el modelo como añadido, con
/// todas sus líneas marcadas como añadidas (sin número de línea viejo).
#[test]
fn ts02_untracked_file_appears_as_added_with_all_lines_added() {
    let (dir, repo) = init_repo();
    let root = dir.path();

    // Estado base: un archivo versionado para que HEAD exista.
    write_file(root, "existing.txt", "x\n");
    stage(&repo, "existing.txt");
    commit_all(&repo, "base");

    // Archivo NUEVO no rastreado (nunca añadido al index).
    write_file(root, "brand_new.txt", "uno\ndos\ntres\n");

    let model = diff::collect(root).expect("collect debe tener éxito");

    let new_file = model
        .files
        .iter()
        .find(|f| f.path.ends_with("brand_new.txt"))
        .expect("el archivo untracked debe estar en el modelo");

    assert_eq!(
        new_file.status,
        FileStatus::Added,
        "un untracked debe modelarse como Added"
    );
    assert_eq!(new_file.additions, 3, "tres líneas añadidas");
    assert_eq!(new_file.deletions, 0, "ninguna eliminación");

    // Todas las líneas del archivo deben ser Added.
    let all_lines: Vec<_> = new_file.hunks.iter().flat_map(|h| &h.lines).collect();
    assert_eq!(all_lines.len(), 3, "tres líneas en total");
    assert!(
        all_lines.iter().all(|l| l.kind == LineKind::Added),
        "todas las líneas deben ser Added"
    );

    // Contenido y numeración: añadidas tienen new_lineno y no old_lineno.
    let contents: Vec<&str> = all_lines.iter().map(|l| l.content.as_str()).collect();
    assert_eq!(contents, vec!["uno", "dos", "tres"]);

    assert_eq!(all_lines[0].old_lineno, None);
    assert_eq!(all_lines[0].new_lineno, Some(1));
    assert_eq!(all_lines[1].new_lineno, Some(2));
    assert_eq!(all_lines[2].new_lineno, Some(3));
}
