//! The two code sheets of a Recommendation No. 20 workbook and the column each field is read from.

use crate::generation_error::GenerationError;
use calamine::Data;

/// A code sheet. The order is the tie-break between rows of equal status: Annex I wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Annex {
    /// Annex I: codes grouped by quantity, with quantity, sector and symbol.
    One,
    /// Annex II and Annex III: every code, without quantity or sector.
    TwoAndThree,
}

/// Column positions of one sheet, zero-based.
#[derive(Debug)]
pub struct Layout {
    /// Sheet name in the workbook.
    pub sheet: &'static str,
    /// Header row, whitespace collapsed, one entry per column.
    pub headers: &'static [&'static str],
    /// Status marker column.
    pub status: usize,
    /// Common code column.
    pub code: usize,
    /// Name column.
    pub name: usize,
    /// Symbol column.
    pub symbol: usize,
    /// Quantity column, `None` on a sheet without one.
    pub quantity: Option<usize>,
    /// Sector column, `None` on a sheet without one.
    pub sector: Option<usize>,
    /// Conversion factor column.
    pub conversion_factor: usize,
    /// Level or category column.
    pub level_category: usize,
    /// Description column.
    pub description: usize,
}

/// Annex I columns of Revision 17.
const ANNEX_ONE: Layout = Layout {
    sheet: "Annex I",
    headers: &[
        "Group Number",
        "Sector",
        "Group ID",
        "Quantity",
        "Level/ Category",
        "Status",
        "Common Code",
        "Name",
        "Conversion Factor",
        "Symbol",
        "Description",
    ],
    status: 5,
    code: 6,
    name: 7,
    symbol: 9,
    quantity: Some(3),
    sector: Some(1),
    conversion_factor: 8,
    level_category: 4,
    description: 10,
};

/// Annex II and Annex III columns of Revision 17.
const ANNEX_TWO_AND_THREE: Layout = Layout {
    sheet: "Annex II & Annex III",
    headers: &[
        "Status",
        "Common Code",
        "Name",
        "Description",
        "Level / Category",
        "Symbol",
        "Conversion Factor",
    ],
    status: 0,
    code: 1,
    name: 2,
    symbol: 5,
    quantity: None,
    sector: None,
    conversion_factor: 6,
    level_category: 4,
    description: 3,
};

impl Annex {
    /// Every code sheet, in reading order.
    pub const ALL: [Annex; 2] = [Annex::One, Annex::TwoAndThree];

    /// Column layout of this sheet.
    pub const fn layout(self) -> &'static Layout {
        match self {
            Annex::One => &ANNEX_ONE,
            Annex::TwoAndThree => &ANNEX_TWO_AND_THREE,
        }
    }
}

impl Layout {
    /// Refuses a header row that does not name every column at the position the layout reads it from.
    pub fn check_headers(&self, header: &[Data]) -> Result<(), GenerationError> {
        self.headers
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
                        sheet: self.sheet,
                        column,
                        expected,
                        found,
                    })
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::annex::Annex;
    use crate::generation_error::GenerationError;
    use calamine::Data;

    fn header(names: &[&str]) -> Vec<Data> {
        names
            .iter()
            .map(|name| Data::String((*name).to_owned()))
            .collect()
    }

    #[test]
    fn a_header_row_matching_the_layout_is_accepted_whatever_its_line_breaks() {
        let row = header(&[
            "Status",
            "Common\nCode",
            "Name",
            "Description",
            "Level /\r\nCategory",
            "Symbol",
            "Conversion Factor",
        ]);
        assert!(Annex::TwoAndThree.layout().check_headers(&row).is_ok());
    }

    #[test]
    fn a_moved_column_fails_the_header_check_naming_the_column() {
        let row = header(&[
            "Common Code",
            "Status",
            "Name",
            "Description",
            "Level / Category",
            "Symbol",
            "Conversion Factor",
        ]);
        let error = Annex::TwoAndThree.layout().check_headers(&row);
        assert!(matches!(
            error,
            Err(GenerationError::UnexpectedHeader {
                column: 0,
                expected: "Status",
                ..
            })
        ));
    }

    #[test]
    fn a_missing_column_fails_the_header_check() {
        let row = header(&["Status", "Common Code"]);
        let error = Annex::TwoAndThree.layout().check_headers(&row);
        assert!(matches!(
            error,
            Err(GenerationError::UnexpectedHeader {
                column: 2,
                expected: "Name",
                ..
            })
        ));
    }

    #[test]
    fn annex_one_wins_a_tie() {
        assert!(Annex::One < Annex::TwoAndThree);
    }
}
