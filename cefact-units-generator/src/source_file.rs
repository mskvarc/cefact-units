//! The workbooks the code list is generated from, each with its recommendation, revision and digest.
//!
//! The sources are every revision of Recommendation No. 20 UNECE publishes as a spreadsheet (4 to 17; revisions 1 to
//! 3 are published as PDF only) and the latest Recommendation No. 21 package type list.

use crate::annex::Annex;
use crate::code_list::CodeList;
use crate::generation_error::GenerationError;
use crate::packaging_row::PACKAGE_TYPE_SHEET;
use crate::packaging_row::PackagingRow;
use crate::workbook_row::WorkbookRow;
use calamine::Data;
use calamine::Range;
use calamine::Reader;
use calamine::Sheets;
use calamine::open_workbook_auto_from_rs;
use sha2::Digest;
use sha2::Sha256;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;

/// Where a source lives in the repository and what it publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpec {
    /// Path relative to the workspace root.
    pub path: &'static str,
    /// Recommendation the workbook publishes.
    pub code_list: CodeList,
    /// Revision of that recommendation.
    pub revision: u16,
}

/// A Recommendation No. 20 source.
const fn recommendation_20(path: &'static str, revision: u16) -> SourceSpec {
    SourceSpec {
        path,
        code_list: CodeList::Recommendation20,
        revision,
    }
}

/// Every source, ascending by recommendation and then by revision.
pub const SOURCES: &[SourceSpec] = &[
    recommendation_20("sources/rec20-rev4.xls", 4),
    recommendation_20("sources/rec20-rev5.xls", 5),
    recommendation_20("sources/rec20-rev6.xls", 6),
    recommendation_20("sources/rec20-rev7.xls", 7),
    recommendation_20("sources/rec20-rev8.xls", 8),
    recommendation_20("sources/rec20-rev9.xls", 9),
    recommendation_20("sources/rec20-rev10.xls", 10),
    recommendation_20("sources/rec20-rev11.xls", 11),
    recommendation_20("sources/rec20-rev12.xls", 12),
    recommendation_20("sources/rec20-rev13.xls", 13),
    recommendation_20("sources/rec20-rev14.xls", 14),
    recommendation_20("sources/rec20-rev15.xls", 15),
    recommendation_20("sources/rec20-rev16.xlsx", 16),
    recommendation_20("sources/rec20-rev17.xlsx", 17),
    SourceSpec {
        path: "sources/rec21-rev11.xls",
        code_list: CodeList::Recommendation21,
        revision: 11,
    },
];

/// A source workbook read into memory.
#[derive(Debug)]
pub struct SourceFile {
    /// Absolute path the workbook was read from.
    pub path: PathBuf,
    /// Recommendation the workbook publishes.
    pub code_list: CodeList,
    /// Revision of that recommendation.
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
            code_list: spec.code_list,
            revision: spec.revision,
            sha256,
            bytes,
        })
    }

    /// Every code row of both Recommendation No. 20 code sheets, Annex I first.
    pub fn rows(&self) -> Result<Vec<WorkbookRow>, GenerationError> {
        let mut workbook = self.workbook()?;
        Annex::ALL
            .into_iter()
            .try_fold(Vec::new(), |mut rows, annex| {
                let range = self.sheet(&mut workbook, annex.layout().sheet)?;
                rows.extend(
                    WorkbookRow::read_sheet(self.revision, annex, &range)
                        .map_err(|source| self.in_source(source))?,
                );
                Ok(rows)
            })
    }

    /// Every code row of the Recommendation No. 21 package type sheet.
    pub fn packaging_rows(&self) -> Result<Vec<PackagingRow>, GenerationError> {
        let mut workbook = self.workbook()?;
        let range = self.sheet(&mut workbook, PACKAGE_TYPE_SHEET)?;
        PackagingRow::read_sheet(&range).map_err(|source| self.in_source(source))
    }

    /// The workbook, `.xls` or `.xlsx`.
    fn workbook(&self) -> Result<Sheets<Cursor<&[u8]>>, GenerationError> {
        open_workbook_auto_from_rs(Cursor::new(self.bytes.as_slice()))
            .map_err(|source| self.workbook_error(source))
    }

    /// The cells of the sheet named `sheet`.
    fn sheet(
        &self,
        workbook: &mut Sheets<Cursor<&[u8]>>,
        sheet: &str,
    ) -> Result<Range<Data>, GenerationError> {
        workbook
            .worksheet_range(sheet)
            .map_err(|source| self.workbook_error(source))
    }

    /// A workbook or sheet that could not be opened.
    fn workbook_error(&self, source: calamine::Error) -> GenerationError {
        GenerationError::Workbook {
            path: self.path.clone(),
            source,
        }
    }

    /// A row failure, attributed to this source.
    fn in_source(&self, source: GenerationError) -> GenerationError {
        GenerationError::InSource {
            path: self.path.clone(),
            source: Box::new(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::code_list::CodeList;
    use crate::source_file::SOURCES;

    #[test]
    fn sources_ascend_by_recommendation_and_revision() {
        assert!(SOURCES.is_sorted_by_key(|spec| (spec.code_list, spec.revision)));
    }

    #[test]
    fn every_recommendation_20_revision_from_4_to_17_is_a_source() {
        let revisions: Vec<u16> = SOURCES
            .iter()
            .filter(|spec| spec.code_list == CodeList::Recommendation20)
            .map(|spec| spec.revision)
            .collect();
        assert_eq!(revisions, (4..=17).collect::<Vec<u16>>());
    }
}
