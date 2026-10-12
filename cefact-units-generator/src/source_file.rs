//! The workbooks the code list is generated from, each with its recommendation, revision and digest.
//!
//! The sources are every revision of Recommendation No. 20 UNECE publishes as a spreadsheet (4 to 17; revisions 1 to
//! 3 are published as PDF only) and every revision of the Recommendation No. 21 package type list published as a
//! spreadsheet (5 to 12; revisions 1 to 4 are PDF only).
//!
//! A package type code is a common code once Recommendation No. 20 can refer to the list holding it, so each
//! Recommendation No. 21 revision is listed from the first Recommendation No. 20 revision published with or after it.
//! The order follows the workbooks' last-saved dates, and three pairs went out together: Recommendation No. 21
//! Revision 5 and Recommendation No. 20 Revision 4 (saved 25 seconds apart in March 2006), Revisions 7 and 6
//! (ECE/TRADE/C/CEFACT/2009/25 and 2009/24) and Revisions 9 and 8 (saved a day apart in November 2012).

use crate::annex::Annex;
use crate::code_list::CodeList;
use crate::generation_error::GenerationError;
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

/// A revision number as UNECE prints it on a workbook: `17`, or `8.1` for the amended Revision 8 of Recommendation
/// No. 21.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRevision {
    /// Revision number.
    pub major: u16,
    /// Amendment number, `0` for an unamended revision.
    pub minor: u16,
}

/// What a source workbook holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceContent {
    /// The Recommendation No. 20 code sheets, Annex I and Annexes II and III.
    UnitCodes,
    /// A Recommendation No. 21 package type sheet.
    PackageTypes {
        /// Sheet name.
        sheet: &'static str,
        /// Revision of Recommendation No. 20 the package type codes are listed from.
        listed_from: u16,
    },
}

/// Where a source lives in the repository and what it publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpec {
    /// Path relative to the workspace root.
    pub path: &'static str,
    /// Revision of the recommendation the workbook publishes.
    pub revision: SourceRevision,
    /// What the workbook holds.
    pub content: SourceContent,
}

impl SourceSpec {
    /// Recommendation the workbook publishes.
    pub const fn code_list(self) -> CodeList {
        match self.content {
            SourceContent::UnitCodes => CodeList::Recommendation20,
            SourceContent::PackageTypes { .. } => CodeList::Recommendation21,
        }
    }

    /// Revision of Recommendation No. 20 the workbook's codes are listed from.
    pub const fn listed_from(self) -> u16 {
        match self.content {
            SourceContent::UnitCodes => self.revision.major,
            SourceContent::PackageTypes { listed_from, .. } => listed_from,
        }
    }
}

/// A Recommendation No. 20 source.
const fn unit_codes(path: &'static str, revision: u16) -> SourceSpec {
    SourceSpec {
        path,
        revision: SourceRevision {
            major: revision,
            minor: 0,
        },
        content: SourceContent::UnitCodes,
    }
}

/// A Recommendation No. 21 source holding its codes on the sheet `Annex V and VI`.
const fn package_types(path: &'static str, major: u16, minor: u16, listed_from: u16) -> SourceSpec {
    SourceSpec {
        path,
        revision: SourceRevision { major, minor },
        content: SourceContent::PackageTypes {
            sheet: "Annex V and VI",
            listed_from,
        },
    }
}

/// Every source, ascending by recommendation and then by revision.
pub const SOURCES: &[SourceSpec] = &[
    unit_codes("sources/rec20-rev4.xls", 4),
    unit_codes("sources/rec20-rev5.xls", 5),
    unit_codes("sources/rec20-rev6.xls", 6),
    unit_codes("sources/rec20-rev7.xls", 7),
    unit_codes("sources/rec20-rev8.xls", 8),
    unit_codes("sources/rec20-rev9.xls", 9),
    unit_codes("sources/rec20-rev10.xls", 10),
    unit_codes("sources/rec20-rev11.xls", 11),
    unit_codes("sources/rec20-rev12.xls", 12),
    unit_codes("sources/rec20-rev13.xls", 13),
    unit_codes("sources/rec20-rev14.xls", 14),
    unit_codes("sources/rec20-rev15.xls", 15),
    unit_codes("sources/rec20-rev16.xlsx", 16),
    unit_codes("sources/rec20-rev17.xlsx", 17),
    // Saved with Recommendation No. 20 Revision 4 (March 2006); its sheet keeps the default name.
    SourceSpec {
        path: "sources/rec21-rev5.xls",
        revision: SourceRevision { major: 5, minor: 0 },
        content: SourceContent::PackageTypes {
            sheet: "Sheet1",
            listed_from: 4,
        },
    },
    // June 2008, after Recommendation No. 20 Revision 5 (April 2008).
    package_types("sources/rec21-rev6.xls", 6, 0, 6),
    package_types("sources/rec21-rev7.xls", 7, 0, 6),
    // July and August 2010, before Recommendation No. 20 Revision 7 (September 2010).
    package_types("sources/rec21-rev8.xls", 8, 0, 7),
    package_types("sources/rec21-rev8.1.xls", 8, 1, 7),
    package_types("sources/rec21-rev9.xls", 9, 0, 8),
    // December 2019 and March 2020, after Recommendation No. 20 Revision 14 (September 2019) and before Revision 15
    // (July 2020).
    package_types("sources/rec21-rev10.xls", 10, 0, 15),
    package_types("sources/rec21-rev11.xls", 11, 0, 15),
    // May 2021, after Recommendation No. 20 Revision 16 (January 2021) and before Revision 17 (June 2021).
    package_types("sources/rec21-rev12.xls", 12, 0, 17),
];

/// A source workbook read into memory.
#[derive(Debug)]
pub struct SourceFile {
    /// What the workbook is.
    pub spec: SourceSpec,
    /// Absolute path the workbook was read from.
    pub path: PathBuf,
    /// SHA-256 of the workbook bytes.
    pub sha256: [u8; 32],
    /// Workbook bytes.
    bytes: Vec<u8>,
}

/// The rows one source publishes.
#[derive(Debug)]
pub struct SourceRows<R> {
    /// The source.
    pub spec: SourceSpec,
    /// Every code row, in reading order.
    pub rows: Vec<R>,
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
            spec,
            path,
            sha256,
            bytes,
        })
    }

    /// Every code row of both Recommendation No. 20 code sheets, Annex I first.
    pub fn rows(&self) -> Result<SourceRows<WorkbookRow>, GenerationError> {
        let mut workbook = self.workbook()?;
        let rows = Annex::ALL
            .into_iter()
            .try_fold(Vec::new(), |mut rows, annex| {
                let range = self.sheet(&mut workbook, annex.layout().sheet)?;
                rows.extend(
                    WorkbookRow::read_sheet(self.spec.revision.major, annex, &range)
                        .map_err(|source| self.in_source(source))?,
                );
                Ok::<_, GenerationError>(rows)
            })?;
        Ok(SourceRows {
            spec: self.spec,
            rows,
        })
    }

    /// Every code row of the Recommendation No. 21 package type sheet `sheet`.
    pub fn packaging_rows(
        &self,
        sheet: &'static str,
    ) -> Result<SourceRows<PackagingRow>, GenerationError> {
        let mut workbook = self.workbook()?;
        let range = self.sheet(&mut workbook, sheet)?;
        let rows =
            PackagingRow::read_sheet(sheet, &range).map_err(|source| self.in_source(source))?;
        Ok(SourceRows {
            spec: self.spec,
            rows,
        })
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
    use crate::source_file::SourceRevision;

    #[test]
    fn sources_ascend_by_recommendation_and_revision() {
        assert!(SOURCES.is_sorted_by_key(|spec| (spec.code_list(), spec.revision)));
    }

    #[test]
    fn every_recommendation_20_revision_from_4_to_17_is_a_source_listed_from_itself() {
        let revisions: Vec<(u16, u16)> = SOURCES
            .iter()
            .filter(|spec| spec.code_list() == CodeList::Recommendation20)
            .map(|spec| (spec.revision.major, spec.listed_from()))
            .collect();
        assert_eq!(
            revisions,
            (4..=17)
                .map(|revision| (revision, revision))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn every_recommendation_21_revision_from_5_to_12_is_listed_from_its_recommendation_20_revision()
    {
        let revisions: Vec<(SourceRevision, u16)> = SOURCES
            .iter()
            .filter(|spec| spec.code_list() == CodeList::Recommendation21)
            .map(|spec| (spec.revision, spec.listed_from()))
            .collect();
        let expected: Vec<(SourceRevision, u16)> = [
            (5, 0, 4),
            (6, 0, 6),
            (7, 0, 6),
            (8, 0, 7),
            (8, 1, 7),
            (9, 0, 8),
            (10, 0, 15),
            (11, 0, 15),
            (12, 0, 17),
        ]
        .into_iter()
        .map(|(major, minor, listed_from)| (SourceRevision { major, minor }, listed_from))
        .collect();
        assert_eq!(revisions, expected);
    }
}
