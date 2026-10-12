//! UN/CEFACT Recommendation No. 20 — Codes for Units of Measure Used in
//! International Trade (Revision 17, 2021).
//!
//! Every code the recommendation lists is included, whatever its status: a
//! deprecated or deleted code stays valid in data written against an earlier
//! revision, and [`UnitInfo::status`] says which it is.
//!
//! The code tables are generated ahead of time from the recommendation's
//! workbook by `cefact-units-generator` and embedded as constant data. No
//! runtime allocations, no I/O.
//!
//! # Example
//!
//! ```rust
//! use cefact_units::UnitOfMeasure;
//!
//! let kg = UnitOfMeasure::from_code("KGM").unwrap();
//! assert_eq!(kg.name(), "kilogram");
//! assert_eq!(kg.symbol(), Some("kg"));
//! assert_eq!(kg.quantity(), Some("mass"));
//!
//! let parsed: UnitOfMeasure = "MTR".parse().unwrap();
//! assert_eq!(parsed.to_string(), "MTR");
//!
//! for unit in UnitOfMeasure::ALL {
//!     println!("{} — {}", unit.code(), unit.name());
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
#![allow(clippy::too_many_lines, clippy::match_same_arms)]

include!("generated.rs");

// ── Status, revision and sources ─────────────────────────────────────────────

/// Status of a code in the latest revision of the recommendation.
///
/// A code listed on several rows takes its most current status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum Status {
    /// Listed for use, including codes marked as added (`+`), renamed (`#`),
    /// changed (`¦`) or reinstated (`=`).
    Current,
    /// Marked `D`: still listed, not to be used in new data.
    Deprecated,
    /// Marked `X`: removed from the recommendation.
    Deleted,
}

/// A revision number of Recommendation No. 20.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Revision(u16);

impl Revision {
    /// The revision number.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A source workbook the code tables are generated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceFile {
    /// Revision of the recommendation the workbook publishes.
    pub revision: Revision,
    /// SHA-256 of the workbook file.
    pub sha256: [u8; 32],
}

// ── UnitCode ─────────────────────────────────────────────────────────────────

impl UnitCode {
    /// Look up a code by its string representation (case-insensitive).
    #[inline]
    #[cfg(feature = "case-insensitive")]
    #[must_use]
    pub fn from_code_ignore_case(code: &str) -> Option<Self> {
        Self::from_code(&code.to_ascii_uppercase())
    }
}

impl core::fmt::Display for UnitCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for UnitCode {
    type Err = UnknownCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[cfg(feature = "case-insensitive")]
        let result = Self::from_code_ignore_case(s);

        #[cfg(not(feature = "case-insensitive"))]
        let result = Self::from_code(s);

        result.ok_or_else(|| UnknownCode(s.to_owned()))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for UnitCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for UnitCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = <&str>::deserialize(deserializer)?;
        code.parse().map_err(serde::de::Error::custom)
    }
}

impl<'a> TryFrom<&'a str> for UnitCode {
    type Error = UnknownCode;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

// ── UnitOfMeasure ────────────────────────────────────────────────────────────

/// All metadata for a [`UnitOfMeasure`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnitInfo {
    /// 2- or 3-character alphanumeric common code (e.g. `"MTR"`, `"KGM"`).
    pub code: &'static str,
    /// English unit name.
    pub name: &'static str,
    /// Unit symbol (e.g. `m`, `kg`), if defined.
    pub symbol: Option<&'static str>,
    /// Physical quantity this unit measures (Annex I only).
    pub quantity: Option<&'static str>,
    /// Sector / scientific discipline (Annex I only).
    pub sector: Option<&'static str>,
    /// Informative conversion factor expression.
    pub conversion_factor: Option<&'static str>,
    /// Normative level/category code from the recommendation.
    pub level_category: Option<&'static str>,
    /// Human-readable description.
    pub description: Option<&'static str>,
    /// Status of the code in the latest revision.
    pub status: Status,
}

impl UnitOfMeasure {
    /// 2- or 3-character common code.
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.info().code
    }

    /// English unit name.
    #[inline]
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.info().name
    }

    /// Unit symbol, if any.
    #[inline]
    #[must_use]
    pub const fn symbol(self) -> Option<&'static str> {
        self.info().symbol
    }

    /// Physical quantity (Annex I only).
    #[inline]
    #[must_use]
    pub const fn quantity(self) -> Option<&'static str> {
        self.info().quantity
    }

    /// Sector / discipline (Annex I only).
    #[inline]
    #[must_use]
    pub const fn sector(self) -> Option<&'static str> {
        self.info().sector
    }

    /// Informative conversion factor expression.
    #[inline]
    #[must_use]
    pub const fn conversion_factor(self) -> Option<&'static str> {
        self.info().conversion_factor
    }

    /// Normative level/category code.
    #[inline]
    #[must_use]
    pub const fn level_category(self) -> Option<&'static str> {
        self.info().level_category
    }

    /// Description text.
    #[inline]
    #[must_use]
    pub const fn description(self) -> Option<&'static str> {
        self.info().description
    }

    /// Status of the code in the latest revision.
    #[inline]
    #[must_use]
    pub const fn status(self) -> Status {
        self.info().status
    }

    /// Look up a unit by its common code (case-sensitive).
    #[inline]
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        UnitCode::from_code(code).map(Self::from_unit_code)
    }

    /// Look up a unit by its common code (case-insensitive).
    #[inline]
    #[cfg(feature = "case-insensitive")]
    #[must_use]
    pub fn from_code_ignore_case(code: &str) -> Option<Self> {
        UnitCode::from_code_ignore_case(code).map(Self::from_unit_code)
    }
}

impl core::fmt::Display for UnitOfMeasure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.code())
    }
}

impl core::str::FromStr for UnitOfMeasure {
    type Err = UnknownCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[cfg(feature = "case-insensitive")]
        let result = Self::from_code_ignore_case(s);

        #[cfg(not(feature = "case-insensitive"))]
        let result = Self::from_code(s);

        result.ok_or_else(|| UnknownCode(s.to_owned()))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for UnitOfMeasure {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for UnitOfMeasure {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = <&str>::deserialize(deserializer)?;
        code.parse().map_err(serde::de::Error::custom)
    }
}

impl<'a> TryFrom<&'a str> for UnitOfMeasure {
    type Error = UnknownCode;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

// ── UnknownCode error ────────────────────────────────────────────────────────

/// Error returned when parsing an unrecognised UN/CEFACT common code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCode(pub String);

impl core::fmt::Display for UnknownCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unknown UN/CEFACT unit code: {:?}", self.0)
    }
}

impl std::error::Error for UnknownCode {}

#[cfg(test)]
mod tests {
    use crate::{LATEST_REVISION, SOURCES, Status, UnitCode, UnitOfMeasure};
    use sha2::{Digest, Sha256};

    const IDENTITY_REGISTRY: &str = include_str!("../data/ids.tsv");
    const FROZEN_PREFIXES: &str = include_str!("../data/ids-frozen.txt");

    fn identity(code: UnitCode) -> u16 {
        code as u16
    }

    #[test]
    fn identities_are_dense_from_one_in_all_order() {
        let out_of_place: Vec<_> = UnitOfMeasure::ALL
            .iter()
            .zip(1u16..)
            .filter(|(unit, expected)| identity(unit.unit_code()) != *expected)
            .map(|(unit, expected)| (unit.code(), expected))
            .collect();
        assert_eq!(out_of_place, []);
    }

    #[test]
    fn the_highest_identity_fits_a_signed_smallint() {
        let highest = UnitOfMeasure::ALL
            .iter()
            .map(|unit| identity(unit.unit_code()))
            .max();
        assert!(highest.is_some_and(|highest| i16::try_from(highest).is_ok()));
    }

    #[test]
    fn identities_carried_over_from_the_broker_registry_are_pinned() {
        for (code, expected) in [
            ("KGM", 1121),
            ("CEL", 367),
            ("MTR", 1407),
            ("KMH", 1134),
            ("A91", 141),
            ("MNJ", 1393),
        ] {
            assert_eq!(
                UnitCode::from_code(code).map(identity),
                Some(expected),
                "{code}"
            );
        }
    }

    #[test]
    fn the_identity_registry_lists_every_code_in_all_order() {
        let listed: Vec<String> = UnitOfMeasure::ALL
            .iter()
            .map(|unit| format!("{}\t{}", identity(unit.unit_code()), unit.code()))
            .collect();
        let registered: Vec<&str> = IDENTITY_REGISTRY.lines().collect();
        assert_eq!(registered, listed);
    }

    #[test]
    fn published_identity_prefixes_are_unchanged() {
        for record in FROZEN_PREFIXES.lines() {
            let fields: Vec<&str> = record.split(' ').collect();
            let [release, count, recorded] = fields[..] else {
                panic!("malformed frozen prefix record {record:?}");
            };
            let count: usize = count.parse().unwrap();
            let prefix: String = IDENTITY_REGISTRY
                .split_inclusive('\n')
                .take(count)
                .collect();
            assert_eq!(prefix.lines().count(), count, "release {release}");
            let recorded: Vec<u8> = (0..recorded.len())
                .step_by(2)
                .map(|at| u8::from_str_radix(&recorded[at..at + 2], 16).unwrap())
                .collect();
            let digest = Sha256::digest(prefix.as_bytes());
            assert_eq!(digest.as_slice(), recorded, "release {release}");
        }
    }

    #[test]
    fn basic_lookup() {
        let kg = UnitOfMeasure::from_code("KGM").unwrap();
        assert_eq!(kg.name(), "kilogram");
        assert_eq!(kg.symbol(), Some("kg"));
    }

    #[test]
    fn from_str_parses() {
        let parsed: UnitOfMeasure = "MTR".parse().unwrap();
        assert_eq!(parsed.code(), "MTR");
    }

    #[test]
    fn unknown_code_fails() {
        assert!(UnitOfMeasure::from_code("INVALID").is_none());
        assert!("INVALID".parse::<UnitOfMeasure>().is_err());
    }

    #[test]
    fn unit_code_roundtrip() {
        let code: UnitCode = "KGM".parse().unwrap();
        assert_eq!(code.as_str(), "KGM");
        let unit = UnitOfMeasure::from_unit_code(code);
        assert_eq!(unit.unit_code(), code);
    }

    #[test]
    fn every_code_of_revision_17_is_listed_whatever_its_status() {
        // Annex II and III list every code of Revision 17 exactly once: 2,136 rows.
        assert_eq!(UnitOfMeasure::ALL.len(), 2136);
    }

    #[test]
    fn a_deprecated_code_is_listed_as_deprecated() {
        let unit = UnitOfMeasure::from_code("A1").unwrap();
        assert_eq!(unit.status(), Status::Deprecated);
    }

    #[test]
    fn a_deleted_code_is_listed_as_deleted() {
        let unit = UnitOfMeasure::from_code("KTM").unwrap();
        assert_eq!(unit.status(), Status::Deleted);
    }

    #[test]
    fn a_code_with_any_current_row_is_current() {
        // A91 has a current and a deprecated row in Annex I.
        let unit = UnitOfMeasure::from_code("A91").unwrap();
        assert_eq!(unit.status(), Status::Current);
    }

    #[test]
    fn a_changed_entry_is_current() {
        // MNJ is current in Annex I and marked `¦` in Annex II.
        let unit = UnitOfMeasure::from_code("MNJ").unwrap();
        assert_eq!(unit.status(), Status::Current);
    }

    #[test]
    fn a_current_code_keeps_its_annex_one_metadata() {
        let unit = UnitOfMeasure::from_code("KGM").unwrap();
        assert_eq!(unit.quantity(), Some("mass"));
        assert_eq!(unit.status(), Status::Current);
    }

    #[test]
    fn the_sources_are_revision_17() {
        assert_eq!(LATEST_REVISION.get(), 17);
        let revisions: Vec<_> = SOURCES.iter().map(|source| source.revision.get()).collect();
        assert_eq!(revisions, [17]);
    }

    #[cfg(feature = "generated-tests")]
    #[test]
    fn every_code_round_trips() {
        for unit in UnitOfMeasure::ALL {
            let code: UnitCode = unit.code().parse().unwrap();
            assert_eq!(code.as_str(), unit.code());
            assert_eq!(UnitOfMeasure::from_unit_code(code), *unit);
            let parsed: UnitOfMeasure = unit.code().parse().unwrap();
            assert_eq!(parsed, *unit);
            assert_eq!(unit.unit_code(), code);
        }
    }

    #[cfg(feature = "case-insensitive")]
    #[test]
    fn case_insensitive_lookup() {
        let kg = UnitOfMeasure::from_code_ignore_case("kgm").unwrap();
        assert_eq!(kg.code(), "KGM");

        let parsed: UnitOfMeasure = "mtr".parse().unwrap();
        assert_eq!(parsed.code(), "MTR");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip_for_unit_of_measure() {
        let unit = UnitOfMeasure::from_code("KGM").unwrap();
        let encoded = serde_json::to_string(&unit).unwrap();
        assert_eq!(encoded, "\"KGM\"");

        let decoded: UnitOfMeasure = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, unit);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip_for_unit_code() {
        let code: UnitCode = "MTR".parse().unwrap();
        let encoded = serde_json::to_string(&code).unwrap();
        assert_eq!(encoded, "\"MTR\"");

        let decoded: UnitCode = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, code);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_writes_a_status_in_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Status::Deprecated).unwrap(),
            "\"deprecated\""
        );
    }
}
