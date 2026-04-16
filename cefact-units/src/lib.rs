//! UN/CEFACT Recommendation No. 20 — Codes for Units of Measure Used in
//! International Trade (Revision 17, 2021).
//!
//! All data is generated at compile time from `rec20.xlsx` and embedded as
//! `static` data. No runtime allocations, no I/O.
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

cefact_units_macros::cefact_units!("rec20.xlsx");

#[cfg(test)]
mod tests {
    use crate::{UnitCode, UnitOfMeasure};

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
}
