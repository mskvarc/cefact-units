//! The committed files the generator owns: writing them and checking that they match the sources.

use crate::generation_error::GenerationError;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// A committed file the generator writes, never edited by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedFile {
    /// Path relative to the workspace root.
    relative_path: &'static str,
}

/// The code tables `cefact-units/src/lib.rs` includes.
pub const UNIT_TABLES: GeneratedFile = GeneratedFile {
    relative_path: "cefact-units/src/generated.rs",
};

/// The identity registry, read as the generator's input and rewritten with every newly listed code appended.
pub const IDENTITY_REGISTRY: GeneratedFile = GeneratedFile {
    relative_path: "cefact-units/data/ids.tsv",
};

impl GeneratedFile {
    /// Absolute path of the file under the workspace `root`.
    pub fn path(self, root: &Path) -> PathBuf {
        root.join(self.relative_path)
    }

    /// Reads the committed file.
    pub fn read(self, root: &Path) -> Result<String, GenerationError> {
        let path = self.path(root);
        fs::read_to_string(&path).map_err(|source| GenerationError::Io { path, source })
    }

    /// Writes `rendered` as the file.
    pub fn write(self, root: &Path, rendered: &str) -> Result<(), GenerationError> {
        let path = self.path(root);
        fs::write(&path, rendered).map_err(|source| GenerationError::Io { path, source })
    }

    /// Fails unless the committed file is byte-for-byte `rendered`.
    pub fn check(self, root: &Path, rendered: &str) -> Result<(), GenerationError> {
        let committed = self.read(root)?;
        verify_fresh(rendered, &committed, self.path(root))
    }
}

/// Everything one generation run renders.
#[derive(Debug)]
pub struct RenderedFiles {
    /// Text of [`UNIT_TABLES`].
    pub unit_tables: String,
    /// Text of [`IDENTITY_REGISTRY`].
    pub identity_registry: String,
}

impl RenderedFiles {
    /// Writes every file.
    pub fn write(&self, root: &Path) -> Result<(), GenerationError> {
        IDENTITY_REGISTRY.write(root, &self.identity_registry)?;
        UNIT_TABLES.write(root, &self.unit_tables)
    }

    /// Fails on the first committed file that differs from its rendering.
    pub fn check(&self, root: &Path) -> Result<(), GenerationError> {
        IDENTITY_REGISTRY.check(root, &self.identity_registry)?;
        UNIT_TABLES.check(root, &self.unit_tables)
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
