//! The committed generated file: writing it and checking that it matches the sources.

use crate::generation_error::GenerationError;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Path of the generated file relative to the workspace root.
const GENERATED_PATH: &str = "cefact-units/src/generated.rs";

/// Absolute path of the generated file under the workspace `root`.
pub fn path(root: &Path) -> PathBuf {
    root.join(GENERATED_PATH)
}

/// Writes `rendered` as the generated file.
pub fn write(root: &Path, rendered: &str) -> Result<(), GenerationError> {
    let path = path(root);
    fs::write(&path, rendered).map_err(|source| GenerationError::Io { path, source })
}

/// Fails unless the committed generated file is byte-for-byte `rendered`.
pub fn check(root: &Path, rendered: &str) -> Result<(), GenerationError> {
    let path = path(root);
    match fs::read_to_string(&path) {
        Ok(committed) => verify_fresh(rendered, &committed, path),
        Err(source) => Err(GenerationError::Io { path, source }),
    }
}

/// Fails with [`GenerationError::Stale`] naming `path` unless `committed` equals `rendered`.
pub fn verify_fresh(rendered: &str, committed: &str, path: PathBuf) -> Result<(), GenerationError> {
    if committed == rendered {
        Ok(())
    } else {
        Err(GenerationError::Stale { path })
    }
}
