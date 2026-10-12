//! The normative relevance a Recommendation No. 20 row gives its code, read from the row's Level/Category cell.
//!
//! The introduction defines levels 1 (normative), 2 (normative equivalent) and 3 (informative). Section VII of the
//! recommendation splits them further: `1S` and `1M` are the standard and the commonly used multiples of level 1, `2S`
//! the multiples of level 2, and `3.1` to `3.9` the categories of level 3. A cell lists one entry, or several on
//! separate lines.

use strum::EnumString;

/// One entry of a Level/Category cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum LevelCategory {
    /// Normative.
    #[strum(serialize = "1")]
    Normative,
    /// A standard multiple of a normative unit.
    #[strum(serialize = "1S")]
    StandardMultiple,
    /// A commonly used multiple of a normative unit.
    #[strum(serialize = "1M")]
    CommonlyUsedMultiple,
    /// Normative equivalent.
    #[strum(serialize = "2")]
    NormativeEquivalent,
    /// A multiple of a normative equivalent unit.
    #[strum(serialize = "2S")]
    NormativeEquivalentMultiple,
    /// Informative.
    #[strum(serialize = "3")]
    Informative,
    /// A qualified base unit of level 1 or 2.
    #[strum(serialize = "3.1")]
    QualifiedBaseUnit,
    /// A sales unit.
    #[strum(serialize = "3.2")]
    SalesUnit,
    /// A packing unit.
    #[strum(serialize = "3.3")]
    PackingUnit,
    /// A shipping and transportation unit.
    #[strum(serialize = "3.4")]
    ShippingAndTransportationUnit,
    /// An industry-specific unit.
    #[strum(serialize = "3.5")]
    IndustrySpecificUnit,
    /// An information technology unit.
    #[strum(serialize = "3.6")]
    InformationTechnologyUnit,
    /// An integer, number or ratio.
    #[strum(serialize = "3.7")]
    IntegerNumberOrRatio,
    /// A multiple, fraction or decimal.
    #[strum(serialize = "3.8")]
    MultipleFractionOrDecimal,
    /// Miscellaneous.
    #[strum(serialize = "3.9")]
    Miscellaneous,
}

impl LevelCategory {
    /// Every entry of the cell text `cell`, one per whitespace-separated token in printed order, `None` when a token is
    /// no entry the recommendation defines.
    pub fn read_cell(cell: &str) -> Option<Vec<LevelCategory>> {
        cell.split_whitespace()
            .map(|token| token.parse().ok())
            .collect()
    }

    /// Path of the library `LevelCategory` variant.
    pub const fn library_path(self) -> &'static str {
        match self {
            LevelCategory::Normative => "LevelCategory::Normative",
            LevelCategory::StandardMultiple => "LevelCategory::StandardMultiple",
            LevelCategory::CommonlyUsedMultiple => "LevelCategory::CommonlyUsedMultiple",
            LevelCategory::NormativeEquivalent => "LevelCategory::NormativeEquivalent",
            LevelCategory::NormativeEquivalentMultiple => {
                "LevelCategory::NormativeEquivalentMultiple"
            }
            LevelCategory::Informative => "LevelCategory::Informative",
            LevelCategory::QualifiedBaseUnit => "LevelCategory::QualifiedBaseUnit",
            LevelCategory::SalesUnit => "LevelCategory::SalesUnit",
            LevelCategory::PackingUnit => "LevelCategory::PackingUnit",
            LevelCategory::ShippingAndTransportationUnit => {
                "LevelCategory::ShippingAndTransportationUnit"
            }
            LevelCategory::IndustrySpecificUnit => "LevelCategory::IndustrySpecificUnit",
            LevelCategory::InformationTechnologyUnit => "LevelCategory::InformationTechnologyUnit",
            LevelCategory::IntegerNumberOrRatio => "LevelCategory::IntegerNumberOrRatio",
            LevelCategory::MultipleFractionOrDecimal => "LevelCategory::MultipleFractionOrDecimal",
            LevelCategory::Miscellaneous => "LevelCategory::Miscellaneous",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::level_category::LevelCategory;

    #[test]
    fn every_defined_token_reads_as_its_entry() {
        let expected = [
            ("1", LevelCategory::Normative),
            ("1S", LevelCategory::StandardMultiple),
            ("1M", LevelCategory::CommonlyUsedMultiple),
            ("2", LevelCategory::NormativeEquivalent),
            ("2S", LevelCategory::NormativeEquivalentMultiple),
            ("3", LevelCategory::Informative),
            ("3.1", LevelCategory::QualifiedBaseUnit),
            ("3.2", LevelCategory::SalesUnit),
            ("3.3", LevelCategory::PackingUnit),
            ("3.4", LevelCategory::ShippingAndTransportationUnit),
            ("3.5", LevelCategory::IndustrySpecificUnit),
            ("3.6", LevelCategory::InformationTechnologyUnit),
            ("3.7", LevelCategory::IntegerNumberOrRatio),
            ("3.8", LevelCategory::MultipleFractionOrDecimal),
            ("3.9", LevelCategory::Miscellaneous),
        ];
        for (token, entry) in expected {
            assert_eq!(
                LevelCategory::read_cell(token),
                Some(vec![entry]),
                "{token:?}"
            );
        }
    }

    #[test]
    fn a_cell_listing_entries_on_separate_lines_reads_each_in_order() {
        assert_eq!(
            LevelCategory::read_cell("3.1\r\n3.4"),
            Some(vec![
                LevelCategory::QualifiedBaseUnit,
                LevelCategory::ShippingAndTransportationUnit
            ])
        );
        assert_eq!(
            LevelCategory::read_cell("1S\n3.5"),
            Some(vec![
                LevelCategory::StandardMultiple,
                LevelCategory::IndustrySpecificUnit
            ])
        );
    }

    #[test]
    fn an_undefined_token_fails_the_cell() {
        for cell in ["4", "3.10", "1m", "3.1\r\n?"] {
            assert_eq!(LevelCategory::read_cell(cell), None, "{cell:?}");
        }
    }
}
