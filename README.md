# cefact-units

[![Crates.io](https://img.shields.io/crates/v/cefact-units.svg)](https://crates.io/crates/cefact-units)
[![Documentation](https://docs.rs/cefact-units/badge.svg)](https://docs.rs/cefact-units)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md)

UN/CEFACT Recommendation No. 20 — Codes for Units of Measure Used in International Trade (Revision 17, 2021).

Every code the recommendation lists is included, whatever its status: a deprecated or deleted code stays valid in data written against an earlier revision, and `UnitInfo::status` says which it is.

The code tables are generated ahead of time from the recommendation's workbook (`sources/`) by `cefact-units-generator` and embedded as constant data. No runtime allocations, no I/O. To regenerate after changing a source, run `cargo run -p cefact-units-generator -- generate`; `-- check` fails when the committed tables are stale.

Every code has a numeric identity, the `#[repr(u16)]` discriminant of its `UnitCode` variant, so `code as u16` reads it. Identities are append-only: a code keeps its identity in every release, an identity is never reused, codes the sources newly list are appended in ascending code order, and no identity exceeds `32767`, so every identity fits a signed 16-bit SQL `SMALLINT`. `UnitOfMeasure::ALL` lists the units in identity order, and `ALL[id - 1]` is the unit whose code has identity `id`. The registry lives in `cefact-units/data/ids.tsv`; `cefact-units/data/ids-frozen.txt` records, for each published release, how many registry lines it covered and their SHA-256, and a test fails if a published prefix ever changes.

## Usage

```toml
[dependencies]
cefact-units = "0.1"
```

```rust
use cefact_units::UnitOfMeasure;

let kg = UnitOfMeasure::from_code("KGM").unwrap();
assert_eq!(kg.name(), "kilogram");
assert_eq!(kg.symbol(), Some("kg"));
assert_eq!(kg.quantity(), Some("mass"));

let parsed: UnitOfMeasure = "MTR".parse().unwrap();
assert_eq!(parsed.to_string(), "MTR");

for unit in UnitOfMeasure::ALL {
    println!("{} — {}", unit.code(), unit.name());
}
```

## Features

- `serde` — Serialize/deserialize `UnitOfMeasure` and `UnitCode` as strings
- `case-insensitive` — Case-insensitive code parsing

## Minimum Supported Rust Version

Rust 1.85 or later (edition 2024).

## Data Attribution

Unit code data is derived from [UN/CEFACT Recommendation No. 20](https://unece.org/trade/uncefact/cl-recommendations) maintained by the United Nations Economic Commission for Europe (UNECE). The data is provided for public use under UNECE's open data policy.

## License

MIT
