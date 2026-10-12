//! Text read out of workbook cells: one cell of a code row, and a sheet's header row.

use crate::generation_error::GenerationError;
use calamine::Data;
use unicode_normalization::UnicodeNormalization;

/// Position of a row, for error reports.
#[derive(Debug, Clone, Copy)]
pub struct RowPosition {
    /// Sheet name.
    pub sheet: &'static str,
    /// One-based row number as the workbook shows it.
    pub row: u32,
}

/// Text of one cell: trimmed, NFC-normalised, with U+2126 OHM SIGN read as U+03A9 GREEK CAPITAL LETTER OMEGA
/// (its canonical decomposition). An empty or blank cell is `None`; a cell of a kind no column carries fails.
pub fn cell_text(
    cells: &[Data],
    column: usize,
    position: RowPosition,
) -> Result<Option<String>, GenerationError> {
    let raw = match cells.get(column) {
        None | Some(Data::Empty) => return Ok(None),
        Some(Data::String(text)) => text.trim().to_owned(),
        Some(Data::Float(number)) => number.to_string(),
        Some(Data::Int(number)) => number.to_string(),
        Some(
            found @ (Data::Bool(_)
            | Data::DateTime(_)
            | Data::DateTimeIso(_)
            | Data::DurationIso(_)
            | Data::Error(_)),
        ) => {
            return Err(GenerationError::UnexpectedCell {
                sheet: position.sheet,
                row: position.row,
                column,
                found: format!("{found:?}"),
            });
        }
    };
    Ok((!raw.is_empty()).then(|| raw.replace('\u{2126}', "\u{03A9}").nfc().collect()))
}

/// Refuses a header row of `sheet` that does not name every column of `expected` at its position. Line breaks and
/// runs of whitespace inside a header cell read as one space.
pub fn check_headers(
    sheet: &'static str,
    expected: &'static [&'static str],
    header: &[Data],
) -> Result<(), GenerationError> {
    expected
        .iter()
        .enumerate()
        .try_for_each(|(column, expected)| {
            let found = header
                .get(column)
                .map(Data::to_string)
                .unwrap_or_default()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if found == *expected {
                Ok(())
            } else {
                Err(GenerationError::UnexpectedHeader {
                    sheet,
                    column,
                    expected,
                    found,
                })
            }
        })
}

#[cfg(test)]
mod tests {
    use crate::generation_error::GenerationError;
    use crate::sheet_text::RowPosition;
    use crate::sheet_text::cell_text;
    use crate::sheet_text::check_headers;
    use calamine::Data;

    const POSITION: RowPosition = RowPosition {
        sheet: "Annex I",
        row: 2,
    };

    fn header(names: &[&str]) -> Vec<Data> {
        names
            .iter()
            .map(|name| Data::String((*name).to_owned()))
            .collect()
    }

    #[test]
    fn cell_text_is_trimmed_and_the_ohm_sign_becomes_omega() {
        let cells = header(&["  ohm  ", "\u{2126}", "   "]);
        assert_eq!(
            cell_text(&cells, 0, POSITION).ok().flatten().as_deref(),
            Some("ohm")
        );
        assert_eq!(
            cell_text(&cells, 1, POSITION).ok().flatten().as_deref(),
            Some("\u{03A9}")
        );
        assert!(matches!(cell_text(&cells, 2, POSITION), Ok(None)));
        assert!(matches!(cell_text(&cells, 3, POSITION), Ok(None)));
    }

    #[test]
    fn a_number_cell_reads_as_its_decimal_text() {
        let cells = [Data::Float(34.0), Data::Int(7)];
        assert_eq!(
            cell_text(&cells, 0, POSITION).ok().flatten().as_deref(),
            Some("34")
        );
        assert_eq!(
            cell_text(&cells, 1, POSITION).ok().flatten().as_deref(),
            Some("7")
        );
    }

    #[test]
    fn a_cell_of_an_unexpected_kind_fails_naming_its_position() {
        let cells = [Data::Bool(true)];
        assert!(matches!(
            cell_text(&cells, 0, POSITION),
            Err(GenerationError::UnexpectedCell {
                sheet: "Annex I",
                row: 2,
                column: 0,
                ..
            })
        ));
    }

    #[test]
    fn a_header_row_matching_the_expected_names_is_accepted_whatever_its_line_breaks() {
        let row = header(&["Status", "Common\nCode", "Level /\r\nCategory"]);
        assert!(
            check_headers(
                "Annex I",
                &["Status", "Common Code", "Level / Category"],
                &row
            )
            .is_ok()
        );
    }

    #[test]
    fn a_moved_column_fails_the_header_check_naming_the_column() {
        let row = header(&["Common Code", "Status"]);
        assert!(matches!(
            check_headers("Annex I", &["Status", "Common Code"], &row),
            Err(GenerationError::UnexpectedHeader {
                sheet: "Annex I",
                column: 0,
                expected: "Status",
                ..
            })
        ));
    }

    #[test]
    fn a_missing_column_fails_the_header_check() {
        let row = header(&["Status"]);
        assert!(matches!(
            check_headers("Annex I", &["Status", "Common Code"], &row),
            Err(GenerationError::UnexpectedHeader {
                column: 1,
                expected: "Common Code",
                ..
            })
        ));
    }
}
