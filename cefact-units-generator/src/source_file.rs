//! The Recommendation No. 20 workbook the code list is generated from, with its revision and digest.

use crate::annex::Annex;
use crate::generation_error::GenerationError;
use crate::workbook_row::WorkbookRow;
use calamine::Reader;
use calamine::Xlsx;
use sha2::Digest;
use sha2::Sha256;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;

/// Where a source lives in the repository and which revision of the recommendation it is.
#[derive(Debug, Clone, Copy)]
pub struct SourceSpec {
    /// Path relative to the workspace root.
    pub path: &'static str,
    /// Revision number of Recommendation No. 20.
    pub revision: u16,
}

/// Revision 17 (2021).
pub const REC20_REVISION_17: SourceSpec = SourceSpec {
    path: "sources/rec20-rev17.xlsx",
    revision: 17,
};

/// A source workbook read into memory.
#[derive(Debug)]
pub struct SourceFile {
    /// Absolute path the workbook was read from.
    pub path: PathBuf,
    /// Revision number of Recommendation No. 20.
    pub revision: u16,
    /// SHA-256 of the workbook bytes.
    pub sha256: [u8; 32],
    /// Workbook bytes.
    bytes: Vec<u8>,
}

impl SourceFile {
    /// Reads the source `spec` names, relative to the workspace `root`.
    pub fn read(root: &Path, spec: SourceSpec) -> Result<SourceFile, GenerationError> {
        let path = root.join(spec.path);
        let bytes = fs::read(&path).map_err(|source| GenerationError::Io {
            path: path.clone(),
            source,
        })?;
        let sha256 = Sha256::digest(&bytes).into();
        Ok(SourceFile {
            path,
            revision: spec.revision,
            sha256,
            bytes,
        })
    }

    /// Every code row of both code sheets, Annex I first.
    pub fn rows(&self) -> Result<Vec<WorkbookRow>, GenerationError> {
        let workbook_error = |source| GenerationError::Workbook {
            path: self.path.clone(),
            source,
        };
        let mut workbook: Xlsx<Cursor<&[u8]>> =
            Xlsx::new(Cursor::new(self.bytes.as_slice())).map_err(workbook_error)?;
        Annex::ALL
            .into_iter()
            .try_fold(Vec::new(), |mut rows, annex| {
                let range = workbook
                    .worksheet_range(annex.layout().sheet)
                    .map_err(workbook_error)?;
                rows.extend(WorkbookRow::read_sheet(annex, &range)?);
                Ok(rows)
            })
    }
}
