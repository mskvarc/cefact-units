//! UN/CEFACT Recommendation No. 20, Codes for Units of Measure Used in International Trade.
//!
//! [`UnitCode`] lists every code any revision of the recommendation lists, whatever its status: a deprecated or
//! deleted code stays valid in data written against an earlier revision. [`UnitCode::status`] gives the status in the
//! latest revision and [`UnitCode::status_in`] the status in any revision from 4 on. Every code has a numeric identity,
//! [`UnitCode::id`], that never changes between releases and fits a signed 16-bit SQL `SMALLINT`.
//!
//! The package type codes of Recommendation No. 21 are listed too, prefixed with `X` (`XAE`, aerosol), since note 2 of
//! the introduction to Recommendation No. 20 makes them common codes. [`UnitCode::code_list`] tells them apart. Each one
//! is listed from the first Recommendation No. 20 revision published with or after the Recommendation No. 21 revision
//! adding it. The note gives each one the description "A unit of count defining the number of" followed by its name in
//! the plural.
//!
//! The code tables are generated ahead of time from the recommendations' workbooks by `cefact-units-generator` and
//! embedded as constant data. The crate is `no_std`, allocates nothing and does no I/O.
//!
//! # Example
//!
//! ```rust
//! use cefact_units::{Revision, Status, UnitCode};
//!
//! let kg: UnitCode = "KGM".parse()?;
//! assert_eq!(kg.info().name, "kilogram");
//! assert_eq!(kg.info().symbol, Some("kg"));
//! assert_eq!(kg.status(), Status::Current);
//! assert_eq!(kg.id().get(), 1121);
//! assert_eq!(UnitCode::from_smallint(kg.id().to_smallint()), Some(kg));
//! assert_eq!(UnitCode::from_code_ignore_case("kgm"), Some(kg));
//! assert_eq!(kg.first_listed(), Revision::new(4));
//! assert_eq!(kg.status_in(Revision::new(9)), Some(Status::Current));
//! # Ok::<(), cefact_units::UnknownCode>(())
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

use core::error::Error;
use core::fmt;
use core::num::NonZeroU16;
use core::str::FromStr;
#[cfg(feature = "serde")]
use serde::de;

include!("generated.rs");

/// Status of a code in a revision of the recommendation.
///
/// A code listed on several rows of one revision takes its most current status. Revisions up to 7 combine markers
/// (`D|`, `+D`, `X|`): `X` outranks `D`, which outranks every other marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum Status {
    /// Listed for use, including codes marked as added (`+`), renamed (`#`), changed (`¦`, or `|` up to revision 7) or
    /// reinstated (`=`).
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
    /// Revision `number`.
    #[inline]
    #[must_use]
    pub const fn new(number: u16) -> Revision {
        Revision(number)
    }

    /// The revision number.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A revision number as UNECE prints it on a source workbook: `17`, or `8.1` for the amended Revision 8 of
/// Recommendation No. 21.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRevision {
    /// Revision number.
    major: u16,
    /// Amendment number, `0` for an unamended revision.
    minor: u16,
}

impl SourceRevision {
    /// The revision number.
    #[inline]
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// The amendment number, `0` for an unamended revision.
    #[inline]
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }
}

impl fmt::Display for SourceRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.minor {
            0 => write!(f, "{}", self.major),
            minor => write!(f, "{}.{minor}", self.major),
        }
    }
}

/// The UN/CEFACT recommendation a code is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum CodeList {
    /// Recommendation No. 20, Codes for Units of Measure Used in International Trade.
    Recommendation20,
    /// Recommendation No. 21, Codes for Passengers, Types of Cargo, Packages and Packaging Materials: a package type
    /// code prefixed with `X`.
    Recommendation21,
}

/// The status a code takes from one revision of Recommendation No. 20 on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct StatusChange {
    /// Revision the status starts in.
    pub revision: Revision,
    /// Status from that revision on, until the next change.
    pub status: Status,
}

/// A source workbook the code tables are generated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SourceFile {
    /// Recommendation the workbook publishes.
    pub code_list: CodeList,
    /// Revision of that recommendation.
    pub revision: SourceRevision,
    /// Revision of Recommendation No. 20 the workbook's codes are listed from: its own revision for Recommendation No.
    /// 20, the first Recommendation No. 20 revision published with or after it for Recommendation No. 21.
    pub listed_from: Revision,
    /// SHA-256 of the workbook file.
    pub sha256: [u8; 32],
}

/// Numeric identity of a [`UnitCode`], from 1 to 32767.
///
/// An identity is assigned once and never reused, so a stored identity names the same code in every later release.
/// Every identity fits a signed 16-bit SQL `SMALLINT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitCodeId(NonZeroU16);

impl UnitCodeId {
    /// The identity as a number.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }

    /// The identity as a signed 16-bit integer, exact since no identity exceeds `32767`.
    #[inline]
    #[must_use]
    pub const fn to_smallint(self) -> i16 {
        self.0.get().cast_signed()
    }

    /// The identity a stored `SMALLINT` holds, `None` for zero or a negative value.
    #[inline]
    #[must_use]
    pub const fn from_smallint(value: i16) -> Option<UnitCodeId> {
        match NonZeroU16::new(value.cast_unsigned()) {
            Some(identity) if value > 0 => Some(UnitCodeId(identity)),
            Some(_) | None => None,
        }
    }
}

impl UnitCode {
    /// The highest identity this release assigns, held by the last code of [`UnitCode::ALL`].
    pub const MAX_ID: UnitCodeId = Self::ALL[Self::ALL.len() - 1].id();

    /// The common code as text.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.info().code
    }

    /// Everything the recommendation records for the code.
    #[inline]
    #[must_use]
    pub const fn info(self) -> &'static UnitInfo {
        &RECORDS[self.position()]
    }

    /// Status of the code in the latest revision.
    #[inline]
    #[must_use]
    pub const fn status(self) -> Status {
        self.info().status
    }

    /// Recommendation the code is drawn from.
    #[inline]
    #[must_use]
    pub const fn code_list(self) -> CodeList {
        self.info().code_list
    }

    /// Earliest revision of Recommendation No. 20 the sources publish that lists the code. The oldest revision
    /// published as a spreadsheet is 4, so a code first listed in revision 4 may be older.
    #[inline]
    #[must_use]
    pub const fn first_listed(self) -> Revision {
        self.info().first_listed
    }

    /// Status of the code in `revision` of Recommendation No. 20, `None` when that revision does not list it or lies
    /// outside the revisions the sources publish (4 to [`LATEST_REVISION`]).
    #[must_use]
    pub fn status_in(self, revision: Revision) -> Option<Status> {
        if revision > LATEST_REVISION {
            return None;
        }
        self.info()
            .history
            .iter()
            .rev()
            .find(|change| change.revision <= revision)
            .map(|change| change.status)
    }

    /// The code's identity.
    #[inline]
    #[must_use]
    pub const fn id(self) -> UnitCodeId {
        // Every discriminant is at least 1, so the minimum plus the discriminant less one is the discriminant itself.
        UnitCodeId(NonZeroU16::MIN.saturating_add((self as u16).saturating_sub(1)))
    }

    /// The code with identity `id`, `None` when a newer release assigned it.
    #[inline]
    #[must_use]
    pub const fn from_id(id: UnitCodeId) -> Option<UnitCode> {
        let position = (id.get() - 1) as usize;
        if position < Self::ALL.len() {
            Some(Self::ALL[position])
        } else {
            None
        }
    }

    /// The code a stored `SMALLINT` names, `None` for zero, a negative value or an identity a newer release assigned.
    #[inline]
    #[must_use]
    pub const fn from_smallint(value: i16) -> Option<UnitCode> {
        match UnitCodeId::from_smallint(value) {
            Some(id) => Self::from_id(id),
            None => None,
        }
    }

    /// The code `code` spells, case-sensitive.
    #[inline]
    #[must_use]
    pub fn from_code(code: &str) -> Option<UnitCode> {
        Self::from_bytes(code.as_bytes())
    }

    /// The code the bytes `code` spell, case-sensitive.
    #[must_use]
    pub fn from_bytes(code: &[u8]) -> Option<UnitCode> {
        CODES_BY_TEXT
            .binary_search_by(|probe| probe.as_str().as_bytes().cmp(code))
            .ok()
            .and_then(|found| CODES_BY_TEXT.get(found))
            .copied()
    }

    /// The code `code` spells with ASCII letters folded to upper case. Every other character must match exactly.
    #[must_use]
    pub fn from_code_ignore_case(code: &str) -> Option<UnitCode> {
        match *code.as_bytes() {
            [first, second] => {
                Self::from_bytes(&[first.to_ascii_uppercase(), second.to_ascii_uppercase()])
            }
            [first, second, third] => Self::from_bytes(&[
                first.to_ascii_uppercase(),
                second.to_ascii_uppercase(),
                third.to_ascii_uppercase(),
            ]),
            _ => None,
        }
    }

    /// Position of the code in the tables written in identity order.
    const fn position(self) -> usize {
        self as usize - 1
    }
}

/// Everything the recommendations record for a [`UnitCode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct UnitInfo {
    /// The common code as text (for example `"MTR"`, `"KGM"`).
    pub code: &'static str,
    /// English unit name.
    pub name: &'static str,
    /// Unit symbol (for example `m`, `kg`), if the recommendation defines one.
    pub symbol: Option<&'static str>,
    /// Physical quantity the unit measures (Annex I only).
    pub quantity: Option<&'static str>,
    /// Sector or scientific discipline (Annex I only).
    pub sector: Option<&'static str>,
    /// Informative conversion factor expression.
    pub conversion_factor: Option<&'static str>,
    /// Normative level or category code.
    pub level_category: Option<&'static str>,
    /// Human-readable description.
    pub description: Option<&'static str>,
    /// Status of the code in the latest revision.
    pub status: Status,
    /// Recommendation the code is drawn from.
    pub code_list: CodeList,
    /// Earliest revision of Recommendation No. 20 the sources publish that lists the code.
    pub first_listed: Revision,
    /// The status at `first_listed` and at every later revision where it changed, in ascending revision order. The
    /// last entry holds `status`.
    pub history: &'static [StatusChange],
}

impl fmt::Display for UnitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for UnitCode {
    type Err = UnknownCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_code(s).ok_or(UnknownCode)
    }
}

impl TryFrom<&str> for UnitCode {
    type Error = UnknownCode;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl AsRef<str> for UnitCode {
    fn as_ref(&self) -> &str {
        self.as_str()
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

/// Reads a common code from a string or a byte string, borrowed or transient.
#[cfg(feature = "serde")]
struct UnitCodeVisitor;

#[cfg(feature = "serde")]
impl de::Visitor<'_> for UnitCodeVisitor {
    type Value = UnitCode;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a UN/CEFACT common code")
    }

    fn visit_str<E>(self, value: &str) -> Result<UnitCode, E>
    where
        E: de::Error,
    {
        UnitCode::from_code(value).ok_or_else(|| E::custom(UnknownCode))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<UnitCode, E>
    where
        E: de::Error,
    {
        UnitCode::from_bytes(value).ok_or_else(|| E::custom(UnknownCode))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for UnitCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(UnitCodeVisitor)
    }
}

/// A text that spells no common code this release lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnknownCode;

impl fmt::Display for UnknownCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown UN/CEFACT common code")
    }
}

impl Error for UnknownCode {}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::CODES_BY_TEXT;
    use crate::CodeList;
    use crate::LATEST_REVISION;
    use crate::RECORDS;
    use crate::Revision;
    use crate::SOURCES;
    use crate::Status;
    use crate::StatusChange;
    use crate::UnitCode;
    use crate::UnknownCode;
    use sha2::Digest;
    use sha2::Sha256;
    use std::borrow::ToOwned;
    use std::format;
    use std::string::String;
    use std::string::ToString;
    use std::vec::Vec;

    const IDENTITY_REGISTRY: &str = include_str!("../data/ids.tsv");
    const FROZEN_PREFIXES: &str = include_str!("../data/ids-frozen.txt");

    fn status_of(code: &str) -> Option<Status> {
        UnitCode::from_code(code).map(UnitCode::status)
    }

    #[test]
    fn every_code_round_trips_through_its_text() {
        let lost: Vec<UnitCode> = UnitCode::ALL
            .iter()
            .copied()
            .filter(|code| UnitCode::from_code(code.as_str()) != Some(*code))
            .collect();
        assert_eq!(lost, []);
    }

    #[test]
    fn every_code_round_trips_through_its_bytes() {
        let lost: Vec<UnitCode> = UnitCode::ALL
            .iter()
            .copied()
            .filter(|code| UnitCode::from_bytes(code.as_str().as_bytes()) != Some(*code))
            .collect();
        assert_eq!(lost, []);
    }

    #[test]
    fn every_code_round_trips_through_its_identity() {
        let lost: Vec<UnitCode> = UnitCode::ALL
            .iter()
            .copied()
            .filter(|code| {
                UnitCode::from_id(code.id()) != Some(*code)
                    || UnitCode::from_smallint(code.id().to_smallint()) != Some(*code)
            })
            .collect();
        assert_eq!(lost, []);
    }

    #[test]
    fn every_record_belongs_to_the_code_at_its_position() {
        let misplaced: Vec<UnitCode> = UnitCode::ALL
            .iter()
            .copied()
            .zip(RECORDS)
            .filter(|(code, record)| {
                code.info() != *record || UnitCode::from_code(record.code) != Some(*code)
            })
            .map(|(code, _)| code)
            .collect();
        assert_eq!(misplaced, []);
        assert_eq!(RECORDS.len(), UnitCode::ALL.len());
    }

    #[test]
    fn the_text_table_lists_every_code_once_in_ascending_byte_order() {
        assert!(CODES_BY_TEXT.is_sorted_by(|left, right| left.as_str() < right.as_str()));
        assert_eq!(CODES_BY_TEXT.len(), UnitCode::ALL.len());
    }

    #[test]
    fn identities_are_dense_from_one_in_all_order() {
        let out_of_place: Vec<(UnitCode, u16)> = UnitCode::ALL
            .iter()
            .copied()
            .zip(1u16..)
            .filter(|(code, expected)| code.id().get() != *expected)
            .collect();
        assert_eq!(out_of_place, []);
        assert_eq!(
            UnitCode::ALL.last().map(|code| code.id()),
            Some(UnitCode::MAX_ID)
        );
    }

    #[test]
    fn the_highest_identity_fits_a_signed_smallint() {
        assert!(i16::try_from(UnitCode::MAX_ID.get()).is_ok());
        assert_eq!(
            i32::from(UnitCode::MAX_ID.to_smallint()),
            i32::from(UnitCode::MAX_ID.get())
        );
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
                UnitCode::from_code(code).map(|code| code.id().get()),
                Some(expected),
                "{code}"
            );
        }
    }

    #[test]
    fn a_non_positive_or_unassigned_smallint_names_no_code() {
        let mut refused = Vec::from([0, -1, i16::MIN]);
        if let Ok(unassigned) = i16::try_from(UnitCode::MAX_ID.get() + 1) {
            refused.push(unassigned);
        }
        let named: Vec<(i16, Option<UnitCode>)> = refused
            .into_iter()
            .map(|value| (value, UnitCode::from_smallint(value)))
            .filter(|(_, code)| code.is_some())
            .collect();
        assert_eq!(named, []);
    }

    #[test]
    fn the_identity_registry_lists_every_code_in_all_order() {
        let listed: Vec<String> = UnitCode::ALL
            .iter()
            .map(|code| format!("{}\t{}", code.id().get(), code))
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
    fn every_code_of_every_source_is_listed_whatever_its_status() {
        // Annexes II and III of Revision 17 list every code of Recommendation No. 20 exactly once, 2,136 rows, and
        // no earlier revision lists a code Revision 17 lacks. Recommendation No. 21 Revision 12 lists 406 package type
        // codes, and no earlier revision lists one it lacks.
        let from = |list: CodeList| {
            UnitCode::ALL
                .iter()
                .filter(|code| code.code_list() == list)
                .count()
        };
        assert_eq!(from(CodeList::Recommendation20), 2136);
        assert_eq!(from(CodeList::Recommendation21), 406);
        assert_eq!(UnitCode::ALL.len(), 2542);
    }

    #[test]
    fn every_history_starts_at_the_first_listing_and_ends_at_the_latest_status() {
        let malformed: Vec<UnitCode> = UnitCode::ALL
            .iter()
            .copied()
            .filter(|code| {
                let info = code.info();
                info.history.first().map(|change| change.revision) != Some(info.first_listed)
                    || info.history.last().map(|change| change.status) != Some(info.status)
                    || !info.history.windows(2).all(|pair| {
                        pair[0].revision < pair[1].revision && pair[0].status != pair[1].status
                    })
                    || info
                        .history
                        .iter()
                        .any(|change| change.revision > LATEST_REVISION)
            })
            .collect();
        assert_eq!(malformed, []);
    }

    #[test]
    fn the_status_in_a_revision_is_the_last_change_up_to_it() {
        // R4 is deprecated in Revision 4 and deleted from Revision 6 on.
        let r4 = UnitCode::R4;
        assert_eq!(
            r4.info().history,
            [
                StatusChange {
                    revision: Revision::new(4),
                    status: Status::Deprecated
                },
                StatusChange {
                    revision: Revision::new(6),
                    status: Status::Deleted
                },
            ]
        );
        assert_eq!(r4.status_in(Revision::new(4)), Some(Status::Deprecated));
        assert_eq!(r4.status_in(Revision::new(5)), Some(Status::Deprecated));
        assert_eq!(r4.status_in(Revision::new(6)), Some(Status::Deleted));
        assert_eq!(r4.status_in(Revision::new(17)), Some(Status::Deleted));
    }

    #[test]
    fn a_reinstated_code_is_deleted_only_before_its_reinstatement() {
        // HH and NA are marked `X` in Revision 4 and current again from Revisions 5 and 6.
        assert_eq!(
            UnitCode::Hh.status_in(Revision::new(4)),
            Some(Status::Deleted)
        );
        assert_eq!(
            UnitCode::Hh.status_in(Revision::new(5)),
            Some(Status::Current)
        );
        assert_eq!(
            UnitCode::Na.status_in(Revision::new(5)),
            Some(Status::Deleted)
        );
        assert_eq!(
            UnitCode::Na.status_in(Revision::new(6)),
            Some(Status::Current)
        );
        assert_eq!(UnitCode::Hh.status(), Status::Current);
    }

    #[test]
    fn a_code_has_no_status_before_its_first_listing() {
        assert_eq!(UnitCode::Mnj.first_listed(), Revision::new(16));
        assert_eq!(UnitCode::Mnj.status_in(Revision::new(15)), None);
        assert_eq!(
            UnitCode::Mnj.status_in(Revision::new(16)),
            Some(Status::Current)
        );
    }

    #[test]
    fn no_revision_outside_the_sources_gives_a_status() {
        assert_eq!(UnitCode::Kgm.first_listed(), Revision::new(4));
        assert_eq!(UnitCode::Kgm.status_in(Revision::new(3)), None);
        assert_eq!(UnitCode::Kgm.status_in(Revision::new(18)), None);
        assert_eq!(
            UnitCode::Kgm.status_in(LATEST_REVISION),
            Some(Status::Current)
        );
    }

    #[test]
    fn revision_nine_lists_its_own_codes_and_the_package_types_of_recommendation_21_revision_9() {
        // Recommendation No. 20 Revision 9 lists 2,091 codes of its own. Recommendation No. 21 Revision 9, published
        // with Recommendation No. 20 Revision 8, lists 378 package type codes.
        let listed = |list: CodeList| {
            UnitCode::ALL
                .iter()
                .filter(|code| {
                    code.code_list() == list && code.status_in(Revision::new(9)).is_some()
                })
                .count()
        };
        assert_eq!(listed(CodeList::Recommendation20), 2091);
        assert_eq!(listed(CodeList::Recommendation21), 378);
    }

    #[test]
    fn package_type_codes_are_listed_with_an_x_prefix() {
        let aerosol = UnitCode::from_code("XAE");
        assert_eq!(aerosol, Some(UnitCode::Xae));
        let info = UnitCode::Xae.info();
        assert_eq!(info.name, "Aerosol");
        assert_eq!(info.code_list, CodeList::Recommendation21);
        assert_eq!(info.status, Status::Current);
        assert_eq!(UnitCode::Xsx.status(), Status::Deleted);
        assert_eq!(UnitCode::X1.code_list(), CodeList::Recommendation20);
    }

    #[test]
    fn a_package_type_code_is_listed_from_the_recommendation_20_revision_its_list_went_out_with() {
        // Recommendation No. 21 Revision 5 went out with Recommendation No. 20 Revision 4, Revisions 6 and 7 before
        // Revision 6, Revisions 8 and 8.1 before Revision 7, and Revisions 10 and 11 before Revision 15.
        for (code, first_listed) in [
            (UnitCode::Xae, 4),
            (UnitCode::Xok, 6),
            (UnitCode::Xoa, 6),
            (UnitCode::Xfe, 7),
            (UnitCode::Xfb, 7),
            (UnitCode::Xo1, 15),
            (UnitCode::Xo7, 15),
        ] {
            assert_eq!(code.first_listed(), Revision::new(first_listed), "{code}");
        }
        assert_eq!(UnitCode::Xo7.status_in(Revision::new(14)), None);
        assert_eq!(
            UnitCode::Xo7.status_in(Revision::new(15)),
            Some(Status::Current)
        );
        assert_eq!(
            UnitCode::Xsx.info().history,
            [StatusChange {
                revision: Revision::new(4),
                status: Status::Deleted
            }]
        );
    }

    #[test]
    fn package_type_codes_take_identities_after_the_recommendation_20_codes() {
        assert_eq!(UnitCode::X1a.id().get(), 2137);
        assert_eq!(UnitCode::Xae.id().get(), 2169);
        let first_package_type = UnitCode::ALL
            .iter()
            .position(|code| code.code_list() == CodeList::Recommendation21);
        assert_eq!(first_package_type, Some(2136));
    }

    #[test]
    fn a_deprecated_code_is_listed_as_deprecated() {
        assert_eq!(status_of("A1"), Some(Status::Deprecated));
    }

    #[test]
    fn a_deleted_code_is_listed_as_deleted() {
        assert_eq!(status_of("KTM"), Some(Status::Deleted));
    }

    #[test]
    fn a_code_with_any_current_row_is_current() {
        // A91 has a current and a deprecated row in Annex I.
        assert_eq!(status_of("A91"), Some(Status::Current));
    }

    #[test]
    fn a_changed_entry_is_current() {
        // MNJ is current in Annex I and marked `¦` in Annex II.
        assert_eq!(status_of("MNJ"), Some(Status::Current));
    }

    #[test]
    fn lookup_refuses_every_malformed_shape() {
        for code in ["", "K", "KGMX", "kgm", "K M", "KG\u{39C}"] {
            assert_eq!(UnitCode::from_code(code), None, "{code:?}");
        }
    }

    #[test]
    fn a_three_byte_text_led_by_a_zero_byte_names_no_two_character_code() {
        assert!(UnitCode::from_code("A1").is_some());
        assert_eq!(UnitCode::from_bytes(b"\0A1"), None);
        assert_eq!(UnitCode::from_bytes(b"A1\0"), None);
    }

    #[test]
    fn case_folded_lookup_folds_ascii_only() {
        assert_eq!(UnitCode::from_code_ignore_case("kgm"), Some(UnitCode::Kgm));
        assert_eq!(UnitCode::from_code_ignore_case("Kgm"), Some(UnitCode::Kgm));
        assert_eq!(
            UnitCode::from_code_ignore_case("a1"),
            UnitCode::from_code("A1")
        );
        assert_eq!(UnitCode::from_code_ignore_case("\u{FF4B}gm"), None);
    }

    #[test]
    fn from_str_is_case_sensitive() {
        assert_eq!("kgm".parse::<UnitCode>(), Err(UnknownCode));
        assert_eq!("KGM".parse::<UnitCode>(), Ok(UnitCode::Kgm));
        assert_eq!(UnitCode::try_from("kgm"), Err(UnknownCode));
    }

    #[test]
    fn display_and_as_ref_write_the_common_code() {
        assert_eq!(UnitCode::Kgm.to_string(), "KGM");
        assert_eq!(AsRef::<str>::as_ref(&UnitCode::Kgm), "KGM");
    }

    #[test]
    fn an_unknown_code_names_the_vocabulary_it_is_missing_from() {
        assert_eq!(UnknownCode.to_string(), "unknown UN/CEFACT common code");
    }

    #[test]
    fn info_carries_the_workbook_metadata() {
        let info = UnitCode::Kgm.info();
        assert_eq!(info.code, "KGM");
        assert_eq!(info.name, "kilogram");
        assert_eq!(info.symbol, Some("kg"));
        assert_eq!(info.quantity, Some("mass"));
        assert_eq!(info.status, Status::Current);
    }

    #[test]
    fn the_sources_are_every_spreadsheet_of_both_recommendations() {
        assert_eq!(LATEST_REVISION.get(), 17);
        let sources: Vec<(CodeList, String, u16)> = SOURCES
            .iter()
            .map(|source| {
                (
                    source.code_list,
                    source.revision.to_string(),
                    source.listed_from.get(),
                )
            })
            .collect();
        let expected: Vec<(CodeList, String, u16)> = (4..=17)
            .map(|revision| (CodeList::Recommendation20, revision.to_string(), revision))
            .chain(
                [
                    ("5", 4),
                    ("6", 6),
                    ("7", 6),
                    ("8", 7),
                    ("8.1", 7),
                    ("9", 8),
                    ("10", 15),
                    ("11", 15),
                    ("12", 17),
                ]
                .into_iter()
                .map(|(revision, listed_from)| {
                    (CodeList::Recommendation21, revision.to_owned(), listed_from)
                }),
            )
            .collect();
        assert_eq!(sources, expected);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trips_through_the_common_code() {
        let encoded = serde_json::to_string(&UnitCode::Kgm).ok();
        assert_eq!(encoded.as_deref(), Some("\"KGM\""));
        let decoded = encoded.and_then(|encoded| serde_json::from_str::<UnitCode>(&encoded).ok());
        assert_eq!(decoded, Some(UnitCode::Kgm));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_decodes_an_escaped_string() {
        let decoded = serde_json::from_str::<UnitCode>("\"K\\u0047M\"").ok();
        assert_eq!(decoded, Some(UnitCode::Kgm));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_decodes_from_a_reader() {
        let decoded = serde_json::from_reader::<_, UnitCode>(&b"\"KGM\""[..]).ok();
        assert_eq!(decoded, Some(UnitCode::Kgm));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_refuses_an_unknown_code() {
        let refused =
            serde_json::from_str::<UnitCode>("\"ZZZZ\"").map_err(|error| error.to_string());
        assert!(
            refused
                .as_ref()
                .is_err_and(|message| message.contains("unknown UN/CEFACT common code")),
            "{refused:?}"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_writes_a_status_in_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Status::Deprecated).ok().as_deref(),
            Some("\"deprecated\"")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_writes_unit_info_with_its_code_as_text() {
        let encoded = serde_json::to_string(&UnitCode::Kgm.info()).ok();
        assert!(
            encoded.as_deref().is_some_and(
                |encoded| encoded.starts_with("{\"code\":\"KGM\",\"name\":\"kilogram\"")
            ),
            "{encoded:?}"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_writes_a_code_list_in_kebab_case() {
        assert_eq!(
            serde_json::to_string(&CodeList::Recommendation21)
                .ok()
                .as_deref(),
            Some("\"recommendation21\"")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_writes_unit_info_with_its_lifecycle() {
        let encoded = serde_json::to_string(&UnitCode::Kgm.info()).ok();
        assert!(
            encoded.as_deref().is_some_and(|encoded| encoded.ends_with(
                "\"status\":\"current\",\"code_list\":\"recommendation20\",\"first_listed\":4,\"history\":[{\"revision\":4,\"status\":\"current\"}]}"
            )),
            "{encoded:?}"
        );
    }
}
