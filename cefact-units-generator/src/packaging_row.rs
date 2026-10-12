//! One code row of the Recommendation No. 21 package type list, read and validated.
//!
//! Recommendation No. 20 introduction note 2, stated in every revision from 4 to 17, makes each Recommendation No. 21
//! package type code a common code once prefixed with `X` (`AE` becomes `XAE`), reserving `X00` to `XZZ` for them.

use crate::generation_error::GenerationError;
use crate::sheet_text::RowPosition;
use crate::sheet_text::cell_text;
use crate::sheet_text::check_headers;
use crate::status::Status;
use calamine::Data;
use calamine::Range;

/// Header row of a package type sheet, whitespace collapsed.
const HEADERS: &[&str] = &["Status", "Code", "Name", "Description", "Numeric code"];

/// Zero-based offset of the header row, below the title and the subtitle.
const HEADER_OFFSET: usize = 2;

/// Status marker column.
const STATUS: usize = 0;

/// Package type code column.
const CODE: usize = 1;

/// Name column.
const NAME: usize = 2;

/// Description column.
const DESCRIPTION: usize = 3;

/// A row of the package type list. Every row is read, whatever its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagingRow {
    /// Status the row's marker gives the code.
    pub status: Status,
    /// The common code: `X` followed by the 2-character package type code.
    pub code: String,
    /// Package type name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
}

impl PackagingRow {
    /// Reads every code row of the package type sheet named `sheet`, after checking its header row.
    pub fn read_sheet(
        sheet: &'static str,
        range: &Range<Data>,
    ) -> Result<Vec<PackagingRow>, GenerationError> {
        let first_row = range
            .start()
            .map(|(row, _)| row)
            .ok_or(GenerationError::EmptySheet { sheet })?;
        let mut rows = range.rows().zip(first_row + 1..).skip(HEADER_OFFSET);
        let (header, _) = rows.next().ok_or(GenerationError::EmptySheet { sheet })?;
        check_headers(sheet, HEADERS, header)?;
        rows.map(|(cells, row)| PackagingRow::read(cells, RowPosition { sheet, row }))
            .collect()
    }

    /// Reads one code row. A row without a code or a name, with an unknown marker or with a code other than 2
    /// characters of `[A-Z0-9]` fails.
    fn read(cells: &[Data], position: RowPosition) -> Result<PackagingRow, GenerationError> {
        let text = |column: usize| cell_text(cells, column, position);
        let package_type = text(CODE)?.ok_or(GenerationError::MissingCell {
            sheet: position.sheet,
            row: position.row,
            column: "code",
        })?;
        if !is_package_type_shape(&package_type) {
            return Err(GenerationError::MalformedCode {
                sheet: position.sheet,
                row: position.row,
                code: package_type,
            });
        }
        let marker = text(STATUS)?.unwrap_or_default();
        let status = Status::from_marker(&marker).ok_or(GenerationError::UnknownStatusMarker {
            sheet: position.sheet,
            row: position.row,
            marker,
        })?;
        let name = text(NAME)?.ok_or(GenerationError::MissingCell {
            sheet: position.sheet,
            row: position.row,
            column: "name",
        })?;
        Ok(PackagingRow {
            status,
            code: format!("X{package_type}"),
            name,
            description: text(DESCRIPTION)?,
        })
    }
}

/// Whether `code` is 2 characters of `[A-Z0-9]`, the shape of a Recommendation No. 21 package type code.
fn is_package_type_shape(code: &str) -> bool {
    code.len() == 2
        && code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use crate::generation_error::GenerationError;
    use crate::packaging_row::PackagingRow;
    use crate::status::Status;
    use calamine::Data;
    use calamine::Range;

    const TITLE: [&str; 5] = [
        "CODED REPRESENTATIONS OF PACKAGE TYPE NAMES",
        "",
        "",
        "",
        "",
    ];
    const SUBTITLE: [&str; 5] = [
        "(UNECE/CEFACT Trade Facilitation Recommendation No.21)",
        "",
        "",
        "",
        "",
    ];
    const HEADERS: [&str; 5] = ["Status", "Code", "Name", "Description", "Numeric code"];

    fn sheet(rows: &[[&str; 5]]) -> Range<Data> {
        let height = u32::try_from(rows.len() + 2).unwrap_or(u32::MAX);
        let mut range = Range::new((0, 0), (height, 4));
        let all = [&TITLE, &SUBTITLE, &HEADERS].into_iter().chain(rows.iter());
        for (row, cells) in (0u32..).zip(all) {
            for (column, text) in (0u32..).zip(cells.iter()) {
                if !text.is_empty() {
                    range.set_value((row, column), Data::String((*text).to_owned()));
                }
            }
        }
        range
    }

    #[test]
    fn a_package_type_code_is_prefixed_with_x() {
        let range = sheet(&[
            ["", "AE", "Aerosol", "", "42 or 43"],
            [
                "+",
                "O7",
                "Wooden pallet  40 cm x 80 cm",
                "Wooden pallet with dimensions 40 cm x 80 cm.",
                "92",
            ],
            ["X", "SX", "Set", "", "11 to 45"],
        ]);
        let rows = PackagingRow::read_sheet("Annex V and VI", &range);
        let read: Vec<_> = rows
            .iter()
            .flatten()
            .map(|row| {
                (
                    row.code.as_str(),
                    row.name.as_str(),
                    row.description.as_deref(),
                    row.status,
                )
            })
            .collect();
        assert_eq!(
            read,
            [
                ("XAE", "Aerosol", None, Status::Current),
                (
                    "XO7",
                    "Wooden pallet  40 cm x 80 cm",
                    Some("Wooden pallet with dimensions 40 cm x 80 cm."),
                    Status::Current
                ),
                ("XSX", "Set", None, Status::Deleted),
            ]
        );
    }

    #[test]
    fn a_moved_column_fails_the_header_check() {
        let mut range = sheet(&[["", "AE", "Aerosol", "", ""]]);
        range.set_value((2, 1), Data::String("Name".to_owned()));
        assert!(matches!(
            PackagingRow::read_sheet("Annex V and VI", &range),
            Err(GenerationError::UnexpectedHeader {
                sheet: "Annex V and VI",
                column: 1,
                expected: "Code",
                ..
            })
        ));
    }

    #[test]
    fn a_code_other_than_two_characters_fails() {
        for code in ["A", "AEX", "ae"] {
            let range = sheet(&[["", code, "Aerosol", "", ""]]);
            assert!(
                matches!(
                    PackagingRow::read_sheet("Annex V and VI", &range),
                    Err(GenerationError::MalformedCode { row: 4, .. })
                ),
                "{code:?}"
            );
        }
    }

    #[test]
    fn an_unknown_marker_fails_naming_the_row() {
        let range = sheet(&[["?", "AE", "Aerosol", "", ""]]);
        assert!(matches!(
            PackagingRow::read_sheet("Annex V and VI", &range),
            Err(GenerationError::UnknownStatusMarker { row: 4, ref marker, .. }) if marker == "?"
        ));
    }

    #[test]
    fn a_row_without_a_name_fails() {
        let range = sheet(&[["", "AE", "", "", ""]]);
        assert!(matches!(
            PackagingRow::read_sheet("Annex V and VI", &range),
            Err(GenerationError::MissingCell {
                row: 4,
                column: "name",
                ..
            })
        ));
    }
}
