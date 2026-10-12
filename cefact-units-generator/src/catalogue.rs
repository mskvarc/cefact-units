//! The merged code list: one record per common code, whatever the number of rows listing it.

use crate::generation_error::GenerationError;
use crate::status::Status;
use crate::workbook_row::WorkbookRow;
use heck::ToPascalCase;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

/// One common code with the metadata of its most current row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRecord {
    /// Common code.
    pub code: String,
    /// `UnitCode` variant name, derived from the code.
    pub code_variant: String,
    /// `UnitOfMeasure` variant name, derived from the name.
    pub unit_variant: String,
    /// Unit name.
    pub name: String,
    /// Unit symbol.
    pub symbol: Option<String>,
    /// Measured quantity.
    pub quantity: Option<String>,
    /// Sector.
    pub sector: Option<String>,
    /// Informative conversion factor.
    pub conversion_factor: Option<String>,
    /// Level or category.
    pub level_category: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// `Current` if any row is current, else `Deprecated` if any row is deprecated, else `Deleted`.
    pub status: Status,
}

/// Every code the sources list, in order of first appearance (Annex I first, then the Annex II and III additions).
#[derive(Debug)]
pub struct Catalogue {
    /// One record per code.
    pub units: Vec<UnitRecord>,
}

/// The rows listing one code, never empty by construction.
#[derive(Debug)]
struct CodeRows {
    /// First row read for the code.
    first: WorkbookRow,
    /// Every later row for the code, in reading order.
    others: Vec<WorkbookRow>,
}

impl Catalogue {
    /// Merges `rows`, in reading order, into one record per code and names the variants.
    ///
    /// `UnitOfMeasure` names are handed out current codes first, then deprecated, then deleted, so a withdrawn code
    /// never takes the plain name of the current unit it was replaced by (`KTM` and `KMT` are both "kilometre").
    pub fn from_rows(rows: Vec<WorkbookRow>) -> Result<Catalogue, GenerationError> {
        let mut ranked: Vec<(usize, MergedRow)> = group_by_code(rows)
            .into_iter()
            .map(CodeRows::merge)
            .enumerate()
            .collect();
        ranked.sort_by_key(|(_, row)| row.status);
        let mut unit_names: HashMap<String, usize> = HashMap::new();
        let mut named: Vec<(usize, MergedRow, String)> = ranked
            .into_iter()
            .map(|(position, row)| {
                let unit_variant = unit_variant(&mut unit_names, &row.name, &row.code);
                (position, row, unit_variant)
            })
            .collect();
        named.sort_by_key(|(position, ..)| *position);
        let mut code_names: HashMap<String, String> = HashMap::new();
        let units = named
            .into_iter()
            .map(|(_, row, unit_variant)| {
                let code_variant = code_variant(&row.code);
                match code_names.entry(code_variant) {
                    Entry::Occupied(taken) => Err(GenerationError::VariantCollision {
                        variant: taken.key().clone(),
                        first: taken.get().clone(),
                        second: row.code,
                    }),
                    Entry::Vacant(free) => {
                        // The map keeps its own copy of the code to name the first holder in a collision report.
                        let code_variant = free.key().clone();
                        free.insert(row.code.clone());
                        Ok(row.into_record(code_variant, unit_variant))
                    }
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Catalogue { units })
    }
}

/// A code's rows reduced to the fields of its most current row and the best description.
#[derive(Debug)]
struct MergedRow {
    /// Common code.
    code: String,
    /// Name of the most current row.
    name: String,
    /// Symbol of the most current row.
    symbol: Option<String>,
    /// Quantity of the most current row.
    quantity: Option<String>,
    /// Sector of the most current row.
    sector: Option<String>,
    /// Conversion factor of the most current row.
    conversion_factor: Option<String>,
    /// Level or category of the most current row.
    level_category: Option<String>,
    /// Description of the most current row, else the first description among the other rows in rank order.
    description: Option<String>,
    /// Status of the most current row.
    status: Status,
}

impl MergedRow {
    /// Completes the record with its variant names.
    fn into_record(self, code_variant: String, unit_variant: String) -> UnitRecord {
        UnitRecord {
            code: self.code,
            code_variant,
            unit_variant,
            name: self.name,
            symbol: self.symbol,
            quantity: self.quantity,
            sector: self.sector,
            conversion_factor: self.conversion_factor,
            level_category: self.level_category,
            description: self.description,
            status: self.status,
        }
    }
}

impl CodeRows {
    /// Takes the metadata of the row ranked first by [`WorkbookRow::rank`], so a code's status is its most current one.
    fn merge(self) -> MergedRow {
        let (primary, mut rest) =
            self.others
                .into_iter()
                .fold((self.first, Vec::new()), |(best, mut rest), row| {
                    if row.rank() < best.rank() {
                        rest.push(best);
                        (row, rest)
                    } else {
                        rest.push(row);
                        (best, rest)
                    }
                });
        rest.sort_by_key(WorkbookRow::rank);
        let description = primary
            .description
            .or_else(|| rest.into_iter().find_map(|row| row.description));
        MergedRow {
            code: primary.code,
            name: primary.name,
            symbol: primary.symbol,
            quantity: primary.quantity,
            sector: primary.sector,
            conversion_factor: primary.conversion_factor,
            level_category: primary.level_category,
            description,
            status: primary.status,
        }
    }
}

/// Groups rows by code, keeping the order in which each code first appears.
fn group_by_code(rows: Vec<WorkbookRow>) -> Vec<CodeRows> {
    let mut positions: HashMap<String, usize> = HashMap::new();
    let mut groups: Vec<CodeRows> = Vec::new();
    for row in rows {
        match positions.get(&row.code) {
            Some(&position) => {
                if let Some(group) = groups.get_mut(position) {
                    group.others.push(row);
                }
            }
            None => {
                // The index needs its own key: the row moves into the group.
                positions.insert(row.code.clone(), groups.len());
                groups.push(CodeRows {
                    first: row,
                    others: Vec::new(),
                });
            }
        }
    }
    groups
}

/// `UnitCode` variant name: the code in Pascal case, prefixed `Code` when it starts with a digit.
fn code_variant(code: &str) -> String {
    let base = code.to_pascal_case();
    if base.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Code{base}")
    } else {
        base
    }
}

/// `UnitOfMeasure` variant name: the name in Pascal case, `_<CODE>` when that is empty or starts with a digit, with
/// the code appended to every later holder of a name already taken.
fn unit_variant(taken: &mut HashMap<String, usize>, name: &str, code: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let base = cleaned.trim().to_pascal_case();
    let base = if base.is_empty() || base.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{code}")
    } else {
        base
    };
    let holders = taken.entry(base.clone()).or_insert(0);
    *holders += 1;
    if *holders == 1 {
        base
    } else {
        format!("{base}{}", code_variant(code))
    }
}

#[cfg(test)]
mod tests {
    use crate::annex::Annex;
    use crate::catalogue::Catalogue;
    use crate::catalogue::UnitRecord;
    use crate::status::Status;
    use crate::workbook_row::WorkbookRow;

    fn row(
        annex: Annex,
        row: u32,
        code: &str,
        status: Status,
        description: Option<&str>,
    ) -> WorkbookRow {
        WorkbookRow {
            annex,
            row,
            status,
            code: code.to_owned(),
            name: format!("{code} at {row}"),
            symbol: None,
            quantity: None,
            sector: None,
            conversion_factor: None,
            level_category: None,
            description: description.map(str::to_owned),
        }
    }

    fn units(rows: Vec<WorkbookRow>) -> Vec<UnitRecord> {
        Catalogue::from_rows(rows)
            .map(|catalogue| catalogue.units)
            .unwrap_or_default()
    }

    #[test]
    fn a_code_on_several_rows_takes_its_most_current_status() {
        let merged = units(vec![
            row(Annex::TwoAndThree, 2, "AA", Status::Current, None),
            row(Annex::TwoAndThree, 3, "AA", Status::Deprecated, None),
            row(Annex::TwoAndThree, 4, "BB", Status::Deprecated, None),
            row(Annex::TwoAndThree, 5, "BB", Status::Deleted, None),
            row(Annex::TwoAndThree, 6, "CC", Status::Deleted, None),
        ]);
        let statuses: Vec<_> = merged
            .iter()
            .map(|unit| (unit.code.as_str(), unit.status))
            .collect();
        assert_eq!(
            statuses,
            [
                ("AA", Status::Current),
                ("BB", Status::Deprecated),
                ("CC", Status::Deleted)
            ]
        );
    }

    #[test]
    fn a_current_row_wins_over_an_earlier_deprecated_one() {
        let merged = units(vec![
            row(Annex::One, 2, "AA", Status::Deprecated, None),
            row(Annex::TwoAndThree, 9, "AA", Status::Current, None),
        ]);
        let names: Vec<_> = merged
            .iter()
            .map(|unit| (unit.name.as_str(), unit.status))
            .collect();
        assert_eq!(names, [("AA at 9", Status::Current)]);
    }

    #[test]
    fn annex_one_wins_between_rows_of_equal_status() {
        let merged = units(vec![
            row(Annex::One, 7, "AA", Status::Current, None),
            row(Annex::TwoAndThree, 2, "AA", Status::Current, None),
        ]);
        let names: Vec<_> = merged.iter().map(|unit| unit.name.as_str()).collect();
        assert_eq!(names, ["AA at 7"]);
    }

    #[test]
    fn a_missing_description_falls_back_to_another_row_even_an_inactive_one() {
        let merged = units(vec![
            row(Annex::One, 2, "AA", Status::Current, None),
            row(
                Annex::TwoAndThree,
                3,
                "AA",
                Status::Deleted,
                Some("deleted text"),
            ),
            row(
                Annex::TwoAndThree,
                4,
                "AA",
                Status::Deprecated,
                Some("deprecated text"),
            ),
        ]);
        let descriptions: Vec<_> = merged
            .iter()
            .map(|unit| unit.description.as_deref())
            .collect();
        assert_eq!(descriptions, [Some("deprecated text")]);
    }

    #[test]
    fn the_most_current_row_keeps_its_own_description() {
        let merged = units(vec![
            row(Annex::One, 2, "AA", Status::Current, Some("own text")),
            row(
                Annex::TwoAndThree,
                3,
                "AA",
                Status::Current,
                Some("other text"),
            ),
        ]);
        let descriptions: Vec<_> = merged
            .iter()
            .map(|unit| unit.description.as_deref())
            .collect();
        assert_eq!(descriptions, [Some("own text")]);
    }

    #[test]
    fn codes_keep_their_order_of_first_appearance() {
        let merged = units(vec![
            row(Annex::One, 2, "ZZ", Status::Current, None),
            row(Annex::TwoAndThree, 3, "AA", Status::Current, None),
            row(Annex::TwoAndThree, 4, "ZZ", Status::Current, None),
        ]);
        let codes: Vec<_> = merged.iter().map(|unit| unit.code.as_str()).collect();
        assert_eq!(codes, ["ZZ", "AA"]);
    }

    #[test]
    fn code_variants_are_the_code_in_pascal_case() {
        let merged = units(vec![
            row(Annex::TwoAndThree, 2, "KGM", Status::Current, None),
            row(Annex::TwoAndThree, 3, "A91", Status::Current, None),
            row(Annex::TwoAndThree, 4, "10", Status::Current, None),
        ]);
        let variants: Vec<_> = merged
            .iter()
            .map(|unit| unit.code_variant.as_str())
            .collect();
        assert_eq!(variants, ["Kgm", "A91", "Code10"]);
    }

    #[test]
    fn a_later_unit_with_a_taken_name_gets_its_code_appended() {
        let mut first = row(Annex::TwoAndThree, 2, "AA", Status::Current, None);
        first.name = "litre".to_owned();
        let mut second = row(Annex::TwoAndThree, 3, "BB", Status::Current, None);
        second.name = "litre".to_owned();
        let mut numeric = row(Annex::TwoAndThree, 4, "CC", Status::Current, None);
        numeric.name = "10 kg".to_owned();
        let merged = units(vec![first, second, numeric]);
        let variants: Vec<_> = merged
            .iter()
            .map(|unit| unit.unit_variant.as_str())
            .collect();
        assert_eq!(variants, ["Litre", "LitreBb", "_CC"]);
    }

    #[test]
    fn a_withdrawn_unit_never_takes_the_plain_name_of_a_current_one() {
        let mut deleted = row(Annex::One, 2, "KTM", Status::Deleted, None);
        deleted.name = "kilometre".to_owned();
        let mut current = row(Annex::One, 3, "KMT", Status::Current, None);
        current.name = "kilometre".to_owned();
        let merged = units(vec![deleted, current]);
        let variants: Vec<_> = merged
            .iter()
            .map(|unit| (unit.code.as_str(), unit.unit_variant.as_str()))
            .collect();
        assert_eq!(variants, [("KTM", "KilometreKtm"), ("KMT", "Kilometre")]);
    }
}
