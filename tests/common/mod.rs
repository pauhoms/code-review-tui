//! Helpers de test compartidos: construcción determinista de repos git temporales.
//!
//! Vive bajo `tests/` (no es producción). Usa `git2` + `tempfile` para crear
//! repos autolimpiables y deterministas (firma/tiempo fijos, sin depender del
//! git config global del usuario).
//!
//! Algunos helpers (p. ej. `stage_removal`) se usan recién en fases posteriores;
//! el allow acota la supresión de dead_code a este módulo de test, dejando el
//! código de producción bajo `-D warnings`.
#![allow(dead_code)]

use std::fs;
use std::path::Path;

use git2::{Repository, Signature, Time};
use tempfile::TempDir;

/// Firma fija para que los commits sean deterministas (sin tiempo real ni
/// config global).
fn fixed_signature() -> Signature<'static> {
    // epoch fijo, offset 0.
    let time = Time::new(1_700_000_000, 0);
    Signature::new("Test Author", "test@example.com", &time).expect("crear Signature fija")
}

/// Crea un repo git temporal vacío (sin commits) y devuelve el TempDir (que se
/// borra solo al salir de scope) junto al Repository abierto.
pub fn init_repo() -> (TempDir, Repository) {
    let dir = tempfile::tempdir().expect("crear TempDir");
    let repo = Repository::init(dir.path()).expect("git init");
    (dir, repo)
}

/// Escribe (o sobrescribe) un archivo dentro del workdir del repo.
pub fn write_file(root: &Path, rel: &str, contents: &str) {
    let full = root.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("crear directorios");
    }
    fs::write(full, contents).expect("escribir archivo");
}

/// Borra un archivo del workdir del repo.
pub fn remove_file(root: &Path, rel: &str) {
    fs::remove_file(root.join(rel)).expect("borrar archivo");
}

/// Hace `git add <rel>` (stage del archivo en el index).
pub fn stage(repo: &Repository, rel: &str) {
    let mut index = repo.index().expect("abrir index");
    index.add_path(Path::new(rel)).expect("index add_path");
    index.write().expect("index write");
}

/// Hace `git rm` sobre el index (registra el borrado en el index).
pub fn stage_removal(repo: &Repository, rel: &str) {
    let mut index = repo.index().expect("abrir index");
    index
        .remove_path(Path::new(rel))
        .expect("index remove_path");
    index.write().expect("index write");
}

/// Commitea el estado actual del index con firma fija. Devuelve nada; deja HEAD
/// apuntando al nuevo commit.
pub fn commit_all(repo: &Repository, message: &str) {
    let mut index = repo.index().expect("abrir index");
    let tree_oid = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_oid).expect("find_tree");
    let sig = fixed_signature();

    let parents = match repo.head() {
        Ok(head) => {
            let commit = head
                .resolve()
                .expect("resolve HEAD")
                .peel_to_commit()
                .expect("peel HEAD a commit");
            vec![commit]
        }
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .expect("crear commit");
}
