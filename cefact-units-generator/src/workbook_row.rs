//! One code row of a Recommendation No. 20 sheet, read and validated.

use crate::annex::Annex;
use crate::annex::Layout;
use crate::generation_error::GenerationError;
use crate::marker_erratum::intended_marker;
use crate::sheet_text::RowPosition;
use crate::sheet_text::cell_text;
use crate::status::Status;
use calamine::Data;
use calamine::Range;

/// A row of a code sheet. Every row is read, whatever its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookRow {
    /// Sheet the row is on.
    pub annex: Annex,
    /// One-based row number as the workbook shows it.
    pub row: u32,
    /// Status the row's marker gives the code.
    pub status: Status,
    /// Common code, 2 or 3 characters of `[A-Z0-9]`.
    pub code: String,
    /// Unit name.
    pub name: String,
    /// Unit symbol.
    pub symbol: Option<String>,
    /// Measured quantity, Annex I only.
    pub quantity: Option<String>,
    /// Sector, Annex I only.
    pub sector: Option<String>,
    /// Informative conversion factor.
    pub conversion_factor: Option<String>,
    /// Level or category.
    pub level_category: Option<String>,
    /// Description.
    pub description: Option<String>,
}

impl WorkbookRow {
    /// Reads every code row of `annex` from its sheet range in `revision`, after checking the header row.
    pub fn read_sheet(
        revision: u16,
        annex: Annex,
        range: &Range<Data>,
    ) -> Result<Vec<WorkbookRow>, GenerationError> {
        let layout = annex.layout();
        let first_row = range
            .start()
            .map(|(row, _)| row)
            .ok_or(GenerationError::EmptySheet {
                sheet: layout.sheet,
            })?;
        let mut rows = range.rows().zip(first_row + 1..);
        let (header, _) = rows.next().ok_or(GenerationError::EmptySheet {
            sheet: layout.sheet,
        })?;
        layout.check_headers(header)?;
        rows.map(|(cells, row)| {
            WorkbookRow::read(
                revision,
                annex,
                layout,
                cells,
                RowPosition {
                    sheet: layout.sheet,
                    row,
                },
            )
        })
        .collect()
    }

    /// Reads one code row. A row without a code or a name, with an unknown marker or with a malformed code fails. A
    /// misprinted marker reads as the marker it stands for.
    fn read(
        revision: u16,
        annex: Annex,
        layout: &Layout,
        cells: &[Data],
        position: RowPosition,
    ) -> Result<WorkbookRow, GenerationError> {
        let text = |column: usize| cell_text(cells, column, position);
        let optional = |column: Option<usize>| column.map_or(Ok(None), text);
        let code = text(layout.code)?.ok_or(GenerationError::MissingCell {
            sheet: position.sheet,
            row: position.row,
            column: "common code",
        })?;
        if !is_common_code_shape(&code) {
            return Err(GenerationError::MalformedCode {
                sheet: position.sheet,
                row: position.row,
                code,
            });
        }
        let marker = text(layout.status)?.unwrap_or_default();
        let status = Status::from_marker(intended_marker(revision, &code, &marker)).ok_or(
            GenerationError::UnknownStatusMarker {
                sheet: position.sheet,
                row: position.row,
                marker,
            },
        )?;
        let name = text(layout.name)?.ok_or(GenerationError::MissingCell {
            sheet: position.sheet,
            row: position.row,
            column: "name",
        })?;
        Ok(WorkbookRow {
            annex,
            row: position.row,
            status,
            code,
            name,
            symbol: text(layout.symbol)?,
            quantity: optional(layout.quantity)?,
            sector: optional(layout.sector)?,
            conversion_factor: text(layout.conversion_factor)?,
            level_category: text(layout.level_category)?,
            description: text(layout.description)?,
        })
    }

    /// Merge rank: the most current status first, then Annex I, then the earlier row.
    pub fn rank(&self) -> (Status, Annex, u32) {
        (self.status, self.annex, self.row)
    }
}

/// Whether `code` is 2 or 3 characters of `[A-Z0-9]`, the shape of a Recommendation No. 20 common code.
pub fn is_common_code_shape(code: &str) -> bool {
    (2..=3).contains(&code.len())
        && code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use crate::annex::Annex;
    use crate::generation_error::GenerationError;
    use crate::status::Status;
    use crate::workbook_row::WorkbookRow;
    use crate::workbook_row::is_common_code_shape;
    use calamine::Data;
    use calamine::Range;

    const HEADERS: [&str; 7] = [
        "Status",
        "Common Code",
        "Name",
        "Description",
        "Level / Category",
        "Symbol",
        "Conversion Factor",
    ];

    fn sheet(rows: &[[&str; 7]]) -> Range<Data> {
        let height = u32::try_from(rows.len()).unwrap_or(u32::MAX);
        let mut range = Range::new((0, 0), (height, 6));
        let all = std::iter::once(&HEADERS).chain(rows.iter());
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
    fn a_row_of_every_status_is_read() {
        let range = sheet(&[
            ["", "KGM", "kilogram", "", "1", "kg", "kg"],
            ["D", "A1", "15 °C calorie", "", "", "", ""],
            ["X", "KTM", "kilometre", "", "", "", ""],
        ]);
        let rows = WorkbookRow::read_sheet(17, Annex::TwoAndThree, &range);
        let statuses: Vec<_> = rows
            .iter()
            .flatten()
            .map(|row| (row.code.as_str(), row.status, row.row))
            .collect();
        assert_eq!(
            statuses,
            [
                ("KGM", Status::Current, 2),
                ("A1", Status::Deprecated, 3),
                ("KTM", Status::Deleted, 4)
            ]
        );
    }

    #[test]
    fn an_unknown_marker_fails_naming_the_sheet_row_and_marker() {
        let range = sheet(&[["?", "KGM", "kilogram", "", "", "", ""]]);
        let error = WorkbookRow::read_sheet(17, Annex::TwoAndThree, &range);
        assert!(matches!(
            error,
            Err(GenerationError::UnknownStatusMarker { sheet: "Annex II & Annex III", row: 2, ref marker })
                if marker == "?"
        ));
    }

    #[test]
    fn a_row_without_a_code_fails() {
        let range = sheet(&[["", "", "kilogram", "", "", "", ""]]);
        let error = WorkbookRow::read_sheet(17, Annex::TwoAndThree, &range);
        assert!(matches!(
            error,
            Err(GenerationError::MissingCell {
                row: 2,
                column: "common code",
                ..
            })
        ));
    }

    #[test]
    fn a_row_without_a_name_fails() {
        let range = sheet(&[["", "KGM", "", "", "", "", ""]]);
        let error = WorkbookRow::read_sheet(17, Annex::TwoAndThree, &range);
        assert!(matches!(
            error,
            Err(GenerationError::MissingCell {
                row: 2,
                column: "name",
                ..
            })
        ));
    }

    #[test]
    fn a_malformed_code_fails() {
        let range = sheet(&[["", "kgm", "kilogram", "", "", "", ""]]);
        let error = WorkbookRow::read_sheet(17, Annex::TwoAndThree, &range);
        assert!(
            matches!(error, Err(GenerationError::MalformedCode { row: 2, ref code, .. }) if code == "kgm")
        );
    }

    #[test]
    fn a_listed_misprint_reads_as_its_intended_marker() {
        let range = sheet(&[["\u{D7}", "KTM", "kilometre", "", "", "", ""]]);
        let in_revision_11 = WorkbookRow::read_sheet(11, Annex::TwoAndThree, &range);
        assert_eq!(
            in_revision_11
                .ok()
                .map(|rows| rows.iter().map(|row| row.status).collect::<Vec<_>>()),
            Some(vec![Status::Deleted])
        );
        let in_revision_13 = WorkbookRow::read_sheet(13, Annex::TwoAndThree, &range);
        assert!(matches!(
            in_revision_13,
            Err(GenerationError::UnknownStatusMarker { ref marker, .. }) if marker == "\u{D7}"
        ));
    }

    #[test]
    fn the_common_code_shape_is_two_or_three_uppercase_letters_or_digits() {
        for code in ["KG", "KGM", "A91", "10", "2N"] {
            assert!(is_common_code_shape(code), "{code}");
        }
        for code in ["", "K", "KGMX", "kgm", "K M", "K-M", "KG\u{39C}"] {
            assert!(!is_common_code_shape(code), "{code:?}");
        }
    }
}
