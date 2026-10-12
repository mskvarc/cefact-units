//! The merged code list: one record per common code, whatever the number of rows and revisions listing it.
//!
//! A Recommendation No. 20 code takes its metadata from the latest revision and its status history from every
//! revision. A Recommendation No. 21 package type code is listed from the oldest Recommendation No. 20 revision on,
//! since introduction note 2 of every revision makes the package type codes common codes, and it takes the status the
//! package type list gives it: the note refers users to that list for "the latest code elements".

use crate::code_list::CodeList;
use crate::generation_error::GenerationError;
use crate::lifecycle::Lifecycle;
use crate::lifecycle::RevisionRows;
use crate::lifecycle::StatusChange;
use crate::packaging_row::PackagingRow;
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
    /// Recommendation the code is drawn from.
    pub source: CodeList,
    /// Status history.
    pub lifecycle: Lifecycle,
}

impl UnitRecord {
    /// Status in the latest revision.
    pub fn status(&self) -> Status {
        self.lifecycle.latest_status()
    }
}

/// Every code the sources list: the Recommendation No. 20 codes in order of first appearance in the latest revision
/// (Annex I first, then the Annex II and III additions), then the package type codes in list order.
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
    /// Builds the code list from the Recommendation No. 20 `revisions`, in ascending revision order, and the package
    /// type `packaging` rows. A code the latest revision no longer lists fails, as does a code that would share its
    /// variant name with another.
    pub fn build(
        mut revisions: Vec<RevisionRows>,
        packaging: Vec<PackagingRow>,
    ) -> Result<Catalogue, GenerationError> {
        let oldest = revisions
            .first()
            .map(|revision| revision.revision)
            .ok_or(GenerationError::NoRecommendation20Source)?;
        let latest = revisions
            .pop()
            .ok_or(GenerationError::NoRecommendation20Source)?;
        let mut traced = Lifecycle::trace(&revisions)?;
        let mut listed: Vec<ListedUnit> = group_by_code(latest.rows)
            .into_iter()
            .map(CodeRows::merge)
            .map(|row| {
                let change = StatusChange {
                    revision: latest.revision,
                    status: row.status,
                };
                let lifecycle = match traced.remove(&row.code) {
                    Some(mut earlier) => {
                        earlier.record(change);
                        earlier
                    }
                    None => Lifecycle::unchanged(change),
                };
                row.into_listed(lifecycle)
            })
            .collect();
        if let Some((code, _)) = traced.into_iter().next() {
            return Err(GenerationError::DroppedCode {
                code,
                revision: latest.revision,
            });
        }
        listed.extend(
            packaging
                .into_iter()
                .map(|row| ListedUnit::package_type(row, oldest)),
        );
        let mut code_names: HashMap<String, String> = HashMap::new();
        let units = listed
            .into_iter()
            .map(|unit| {
                let code_variant = code_variant(&unit.code);
                match code_names.entry(code_variant) {
                    Entry::Occupied(taken) => Err(GenerationError::VariantCollision {
                        variant: taken.key().clone(),
                        first: taken.get().clone(),
                        second: unit.code,
                    }),
                    Entry::Vacant(free) => {
                        // The map keeps its own copy of the code to name the first holder in a collision report.
                        let code_variant = free.key().clone();
                        free.insert(unit.code.clone());
                        Ok(unit.into_record(code_variant))
                    }
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Catalogue { units })
    }
}

/// A code record before its variant is named.
#[derive(Debug)]
struct ListedUnit {
    /// Common code.
    code: String,
    /// Unit name.
    name: String,
    /// Unit symbol.
    symbol: Option<String>,
    /// Measured quantity.
    quantity: Option<String>,
    /// Sector.
    sector: Option<String>,
    /// Informative conversion factor.
    conversion_factor: Option<String>,
    /// Level or category.
    level_category: Option<String>,
    /// Description.
    description: Option<String>,
    /// Recommendation the code is drawn from.
    source: CodeList,
    /// Status history.
    lifecycle: Lifecycle,
}

impl ListedUnit {
    /// A package type code, listed from the Recommendation No. 20 revision `oldest` on in the status its list gives.
    fn package_type(row: PackagingRow, oldest: u16) -> ListedUnit {
        ListedUnit {
            code: row.code,
            name: row.name,
            symbol: None,
            quantity: None,
            sector: None,
            conversion_factor: None,
            level_category: None,
            description: row.description,
            source: CodeList::Recommendation21,
            lifecycle: Lifecycle::unchanged(StatusChange {
                revision: oldest,
                status: row.status,
            }),
        }
    }

    /// Completes the record with its variant name.
    fn into_record(self, code_variant: String) -> UnitRecord {
        UnitRecord {
            code: self.code,
            code_variant,
            name: self.name,
            symbol: self.symbol,
            quantity: self.quantity,
            sector: self.sector,
            conversion_factor: self.conversion_factor,
            level_category: self.level_category,
            description: self.description,
            source: self.source,
            lifecycle: self.lifecycle,
        }
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
    /// The Recommendation No. 20 code with its status history.
    fn into_listed(self, lifecycle: Lifecycle) -> ListedUnit {
        ListedUnit {
            code: self.code,
            name: self.name,
            symbol: self.symbol,
            quantity: self.quantity,
            sector: self.sector,
            conversion_factor: self.conversion_factor,
            level_category: self.level_category,
            description: self.description,
            source: CodeList::Recommendation20,
            lifecycle,
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

#[cfg(test)]
mod tests {
    use crate::annex::Annex;
    use crate::catalogue::Catalogue;
    use crate::catalogue::UnitRecord;
    use crate::code_list::CodeList;
    use crate::generation_error::GenerationError;
    use crate::lifecycle::RevisionRows;
    use crate::lifecycle::StatusChange;
    use crate::packaging_row::PackagingRow;
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
        Catalogue::build(vec![RevisionRows { revision: 17, rows }], Vec::new())
            .map(|catalogue| catalogue.units)
            .unwrap_or_default()
    }

    fn package_type(code: &str, name: &str, status: Status) -> PackagingRow {
        PackagingRow {
            status,
            code: code.to_owned(),
            name: name.to_owned(),
            description: None,
        }
    }

    const fn change(revision: u16, status: Status) -> StatusChange {
        StatusChange { revision, status }
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
            .map(|unit| (unit.code.as_str(), unit.status()))
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
            .map(|unit| (unit.name.as_str(), unit.status()))
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
    fn a_code_takes_its_metadata_from_the_latest_revision_and_its_history_from_every_revision() {
        let mut earlier = row(
            Annex::TwoAndThree,
            2,
            "AA",
            Status::Current,
            Some("old text"),
        );
        earlier.name = "old name".to_owned();
        let catalogue = Catalogue::build(
            vec![
                RevisionRows {
                    revision: 9,
                    rows: vec![earlier],
                },
                RevisionRows {
                    revision: 17,
                    rows: vec![row(Annex::TwoAndThree, 2, "AA", Status::Deprecated, None)],
                },
            ],
            Vec::new(),
        );
        let read: Vec<_> = catalogue
            .iter()
            .flat_map(|catalogue| &catalogue.units)
            .map(|unit| {
                (
                    unit.name.as_str(),
                    unit.description.as_deref(),
                    unit.status(),
                    unit.lifecycle.history().collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            read,
            [(
                "AA at 2",
                None,
                Status::Deprecated,
                vec![change(9, Status::Current), change(17, Status::Deprecated)]
            )]
        );
    }

    #[test]
    fn a_code_the_latest_revision_drops_fails_generation() {
        let catalogue = Catalogue::build(
            vec![
                RevisionRows {
                    revision: 16,
                    rows: vec![
                        row(Annex::TwoAndThree, 2, "AA", Status::Current, None),
                        row(Annex::TwoAndThree, 3, "BB", Status::Current, None),
                    ],
                },
                RevisionRows {
                    revision: 17,
                    rows: vec![row(Annex::TwoAndThree, 2, "BB", Status::Current, None)],
                },
            ],
            Vec::new(),
        );
        assert!(matches!(
            catalogue,
            Err(GenerationError::DroppedCode { ref code, revision: 17 }) if code == "AA"
        ));
    }

    #[test]
    fn package_type_codes_are_listed_from_the_oldest_revision_in_their_list_status() {
        let catalogue = Catalogue::build(
            vec![
                RevisionRows {
                    revision: 4,
                    rows: vec![row(Annex::TwoAndThree, 2, "KGM", Status::Current, None)],
                },
                RevisionRows {
                    revision: 17,
                    rows: vec![row(Annex::TwoAndThree, 2, "KGM", Status::Current, None)],
                },
            ],
            vec![
                package_type("XAE", "Aerosol", Status::Current),
                package_type("XSX", "Set", Status::Deleted),
            ],
        );
        let read: Vec<_> = catalogue
            .iter()
            .flat_map(|catalogue| &catalogue.units)
            .map(|unit| {
                (
                    unit.code.as_str(),
                    unit.code_variant.as_str(),
                    unit.source,
                    unit.lifecycle.history().collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            read,
            [
                (
                    "KGM",
                    "Kgm",
                    CodeList::Recommendation20,
                    vec![change(4, Status::Current)]
                ),
                (
                    "XAE",
                    "Xae",
                    CodeList::Recommendation21,
                    vec![change(4, Status::Current)]
                ),
                (
                    "XSX",
                    "Xsx",
                    CodeList::Recommendation21,
                    vec![change(4, Status::Deleted)]
                ),
            ]
        );
    }

    #[test]
    fn a_package_type_code_spelling_a_recommendation_20_code_fails_generation() {
        let catalogue = Catalogue::build(
            vec![RevisionRows {
                revision: 17,
                rows: vec![row(Annex::TwoAndThree, 2, "XAE", Status::Current, None)],
            }],
            vec![package_type("XAE", "Aerosol", Status::Current)],
        );
        assert!(matches!(
            catalogue,
            Err(GenerationError::VariantCollision { ref first, ref second, .. }) if first == "XAE" && second == "XAE"
        ));
    }

    #[test]
    fn no_recommendation_20_source_fails_generation() {
        assert!(matches!(
            Catalogue::build(Vec::new(), Vec::new()),
            Err(GenerationError::NoRecommendation20Source)
        ));
    }
}
