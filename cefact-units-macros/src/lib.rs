//! Proc-macro for generating UN/CEFACT unit code types from rec20.xlsx.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use calamine::{open_workbook, Data, Reader, Xlsx};
use heck::ToPascalCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, LitStr};
use unicode_normalization::UnicodeNormalization;

// ── data model ───────────────────────────────────────────────────────────────

struct Record {
    variant: String,
    code_variant: String,
    code: String,
    name: String,
    symbol: Option<String>,
    quantity: Option<String>,
    sector: Option<String>,
    conversion_factor: Option<String>,
    level_category: Option<String>,
    description: Option<String>,
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn cell(data: &Data) -> Option<String> {
    match data {
        Data::String(s) => {
            let t = s.trim().to_owned();
            (!t.is_empty()).then_some(t)
        }
        Data::Float(f) => Some(f.to_string()),
        Data::Int(i) => Some(i.to_string()),
        _ => None,
    }
}

fn is_active(status_cell: &Data) -> bool {
    !matches!(
        cell(status_cell).as_deref(),
        Some("D") | Some("X") | Some("¦")
    )
}

fn pascal(name: &str, code: &str) -> String {
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
    if base.is_empty() || base.starts_with(|c: char| c.is_ascii_digit()) {
        format!(
            "_{}",
            code.chars()
                .map(|c| if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                })
                .collect::<String>()
        )
    } else {
        base
    }
}

fn code_suffix_pascal(code: &str) -> String {
    let cleaned: String = code
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let pascal = cleaned.to_pascal_case();
    if pascal.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Code{pascal}")
    } else {
        pascal
    }
}

fn code_variant(code: &str) -> String {
    let cleaned: String = code
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let base = cleaned.trim().to_pascal_case();
    if base.is_empty() || base.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Code{base}")
    } else {
        base
    }
}

fn normalize_text(value: String) -> String {
    // U+2126 (OHM SIGN) → U+03A9 (GREEK CAPITAL LETTER OMEGA)
    value.replace('\u{2126}', "\u{03A9}").nfc().collect()
}

fn test_fn_name(code: &str) -> String {
    let sanitized: String = code
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if sanitized.starts_with(|c: char| c.is_ascii_digit()) {
        format!("test_code_{}", sanitized)
    } else {
        format!("test_{}", sanitized)
    }
}

// ── parsing ───────────────────────────────────────────────────────────────────

fn parse(wb: &mut Xlsx<BufReader<File>>) -> Vec<Record> {
    let mut records: Vec<Record> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Annex I columns:
    // 0 Group Number | 1 Sector | 2 Group ID | 3 Quantity | 4 Level/Category
    // 5 Status | 6 Common Code | 7 Name | 8 Conversion Factor | 9 Symbol | 10 Description
    {
        let sheet = wb
            .worksheet_range("Annex I")
            .expect("sheet 'Annex I' missing");
        let get = |row: &[Data], i: usize| row.get(i).and_then(cell).map(normalize_text);

        for row in sheet.rows().skip(1) {
            if !is_active(row.get(5).unwrap_or(&Data::Empty)) {
                continue;
            }
            let Some(code) = get(row, 6) else {
                continue;
            };
            if seen.contains(&code) {
                continue;
            }
            seen.insert(code.clone());
            records.push(Record {
                variant: String::new(),
                code_variant: String::new(),
                code,
                name: get(row, 7).unwrap_or_default(),
                symbol: get(row, 9),
                quantity: get(row, 3),
                sector: get(row, 1),
                conversion_factor: get(row, 8),
                level_category: get(row, 4),
                description: get(row, 10),
            });
        }
    }

    // Annex II & III columns:
    // 0 Status | 1 Common Code | 2 Name | 3 Description
    // 4 Level/Category | 5 Symbol | 6 Conversion Factor
    let mut desc_supplement: HashMap<String, String> = HashMap::new();
    {
        let sheet = wb
            .worksheet_range("Annex II & Annex III")
            .expect("sheet 'Annex II & Annex III' missing");
        let get = |row: &[Data], i: usize| row.get(i).and_then(cell).map(normalize_text);

        for row in sheet.rows().skip(1) {
            if !is_active(row.first().unwrap_or(&Data::Empty)) {
                continue;
            }
            let Some(code) = get(row, 1) else {
                continue;
            };

            if let Some(desc) = get(row, 3) {
                desc_supplement.entry(code.clone()).or_insert(desc);
            }

            if seen.contains(&code) {
                continue;
            }
            seen.insert(code.clone());
            records.push(Record {
                variant: String::new(),
                code_variant: String::new(),
                code,
                name: get(row, 2).unwrap_or_default(),
                symbol: get(row, 5),
                quantity: None,
                sector: None,
                conversion_factor: get(row, 6),
                level_category: get(row, 4),
                description: get(row, 3),
            });
        }
    }

    // Fill missing descriptions from supplement map.
    for rec in &mut records {
        if rec.description.is_none() {
            rec.description = desc_supplement.get(&rec.code).cloned();
        }
    }

    // Assign unique PascalCase variant names.
    let mut counts: HashMap<String, usize> = HashMap::new();
    for rec in &mut records {
        let base = pascal(&rec.name, &rec.code);
        let n = counts.entry(base.clone()).or_insert(0);
        rec.variant = if *n == 0 {
            base
        } else {
            let suffix = code_suffix_pascal(&rec.code);
            format!("{base}{suffix}")
        };
        *n += 1;
    }

    let mut code_variant_counts: HashMap<String, usize> = HashMap::new();
    for rec in &mut records {
        let base = code_variant(&rec.code);
        let n = code_variant_counts.entry(base.clone()).or_insert(0);
        rec.code_variant = if *n == 0 {
            base
        } else {
            let suffix = code_suffix_pascal(&rec.code);
            format!("{base}{suffix}")
        };
        *n += 1;
    }

    records
}

fn opt_str(v: &Option<String>) -> proc_macro2::TokenStream {
    match v {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    }
}

/// Generate UN/CEFACT unit types from an Excel file.
///
/// Usage: `cefact_units!("rec20.xlsx");`
#[proc_macro]
pub fn cefact_units(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let rel_path = path_lit.value();

    // Resolve path relative to calling crate's CARGO_MANIFEST_DIR
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let xlsx_path = PathBuf::from(&manifest_dir).join(&rel_path);

    let mut wb: Xlsx<_> = open_workbook(&xlsx_path)
        .unwrap_or_else(|e| panic!("cannot open {}: {e}", xlsx_path.display()));

    let records = parse(&mut wb);

    // Generate identifiers
    let unit_variants: Vec<_> = records
        .iter()
        .map(|r| format_ident!("{}", r.variant))
        .collect();
    let code_variants: Vec<_> = records
        .iter()
        .map(|r| format_ident!("{}", r.code_variant))
        .collect();
    let codes: Vec<_> = records.iter().map(|r| &r.code).collect();
    let codes_upper: Vec<_> = records.iter().map(|r| r.code.to_ascii_uppercase()).collect();
    let names: Vec<_> = records.iter().map(|r| &r.name).collect();
    let symbols: Vec<_> = records.iter().map(|r| opt_str(&r.symbol)).collect();
    let quantities: Vec<_> = records.iter().map(|r| opt_str(&r.quantity)).collect();
    let sectors: Vec<_> = records.iter().map(|r| opt_str(&r.sector)).collect();
    let conversion_factors: Vec<_> = records
        .iter()
        .map(|r| opt_str(&r.conversion_factor))
        .collect();
    let level_categories: Vec<_> = records
        .iter()
        .map(|r| opt_str(&r.level_category))
        .collect();
    let descriptions: Vec<_> = records.iter().map(|r| opt_str(&r.description)).collect();
    let count = records.len();

    let code_docs: Vec<_> = records
        .iter()
        .map(|r| format!("Code `{}`.", r.code))
        .collect();
    let unit_docs: Vec<_> = records
        .iter()
        .map(|r| format!("Unit `{}`.", r.code))
        .collect();
    let test_fns: Vec<_> = records
        .iter()
        .map(|r| format_ident!("{}", test_fn_name(&r.code)))
        .collect();

    let output = quote! {
        // ── UnitCode enum ────────────────────────────────────────────────────────

        /// Canonical UN/CEFACT common code (for example: `"MTR"`, `"KGM"`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[non_exhaustive]
        pub enum UnitCode {
            #(
                #[doc = #code_docs]
                #code_variants,
            )*
        }

        impl UnitCode {
            /// Returns this code as a static string.
            #[inline]
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    #( Self::#code_variants => #codes, )*
                }
            }

            /// Look up a code by its string representation (case-sensitive).
            #[inline]
            #[must_use]
            pub fn from_code(code: &str) -> Option<Self> {
                match code {
                    #( #codes => Some(Self::#code_variants), )*
                    _ => None,
                }
            }

            /// Look up a code by its string representation (case-insensitive).
            #[inline]
            #[cfg(feature = "case-insensitive")]
            #[must_use]
            pub fn from_code_ignore_case(code: &str) -> Option<Self> {
                match code.to_ascii_uppercase().as_str() {
                    #( #codes_upper => Some(Self::#code_variants), )*
                    _ => None,
                }
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

        // ── UnitOfMeasure enum ───────────────────────────────────────────────────

        /// Every active UN/CEFACT unit of measure code (Rec 20, Rev 17 — 2021).
        ///
        /// Use [`UnitOfMeasure::from_code`] to look up by common code,
        /// or iterate [`UnitOfMeasure::ALL`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[non_exhaustive]
        pub enum UnitOfMeasure {
            #(
                #[doc = #unit_docs]
                #unit_variants,
            )*
        }

        // ── UnitInfo struct ──────────────────────────────────────────────────────

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
        }

        impl UnitOfMeasure {
            /// Returns all static metadata for this unit.
            #[inline]
            #[must_use]
            pub const fn info(self) -> UnitInfo {
                match self {
                    #(
                        Self::#unit_variants => UnitInfo {
                            code: #codes,
                            name: #names,
                            symbol: #symbols,
                            quantity: #quantities,
                            sector: #sectors,
                            conversion_factor: #conversion_factors,
                            level_category: #level_categories,
                            description: #descriptions,
                        },
                    )*
                }
            }

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

            /// Returns this unit's strongly-typed code.
            #[inline]
            #[must_use]
            pub const fn unit_code(self) -> UnitCode {
                match self {
                    #( Self::#unit_variants => UnitCode::#code_variants, )*
                }
            }

            /// Converts a code into its associated unit.
            #[inline]
            #[must_use]
            pub const fn from_unit_code(code: UnitCode) -> Self {
                match code {
                    #( UnitCode::#code_variants => Self::#unit_variants, )*
                }
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

            /// Every active unit in source order (Annex I first, then Annex II/III additions).
            pub const ALL: &'static [Self; #count] = &[
                #( Self::#unit_variants, )*
            ];
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

        // ── UnknownCode error ────────────────────────────────────────────────────

        /// Error returned when parsing an unrecognised UN/CEFACT common code.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct UnknownCode(pub String);

        impl core::fmt::Display for UnknownCode {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "unknown UN/CEFACT unit code: {:?}", self.0)
            }
        }

        impl std::error::Error for UnknownCode {}

        // ── Generated tests ──────────────────────────────────────────────────────

        #[cfg(all(test, feature = "generated-tests"))]
        mod generated_tests {
            use super::*;
            #(
                #[test]
                fn #test_fns() {
                    let code: UnitCode = #codes.parse().unwrap();
                    assert_eq!(code.as_str(), #codes);
                    let unit = UnitOfMeasure::from_unit_code(code);
                    assert_eq!(unit.code(), #codes);
                    let parsed: UnitOfMeasure = #codes.parse().unwrap();
                    assert_eq!(parsed.code(), #codes);
                    assert_eq!(unit.unit_code(), code);
                    let _ = unit.info();
                }
            )*
        }
    };

    output.into()
}
