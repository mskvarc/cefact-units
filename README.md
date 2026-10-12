# cefact-units

[![Crates.io](https://img.shields.io/crates/v/cefact-units.svg)](https://crates.io/crates/cefact-units)
[![Documentation](https://docs.rs/cefact-units/badge.svg)](https://docs.rs/cefact-units)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md)

UN/CEFACT Recommendation No. 20 — Codes for Units of Measure Used in International Trade (Revision 17, 2021).

`UnitCode` lists every code the recommendation lists, whatever its status. `UnitCode::status` says which status a code has in the latest revision: `Current` (including codes marked added, renamed, changed or reinstated), `Deprecated` (marked `D`: still listed, not to be used in new data) or `Deleted` (marked `X`: removed from the recommendation). A deprecated or deleted code stays valid in data written against an earlier revision, so the crate never drops one.

The code tables are generated ahead of time from the recommendation's workbook (`sources/`) by `cefact-units-generator` and embedded as constant data. `LATEST_REVISION` and `SOURCES` name the revision and the SHA-256 of each source workbook. The crate is `no_std`, allocates nothing and does no I/O. To regenerate after changing a source, run `cargo run -p cefact-units-generator -- generate`; `-- check` fails when the committed tables are stale.

Every code has a numeric identity, `UnitCode::id()`, a `UnitCodeId` from 1 to 32767. Identities are append-only: a code keeps its identity in every release, an identity is never reused, codes the sources newly list are appended in ascending code order, and no identity exceeds `32767`, so every identity fits a signed 16-bit SQL `SMALLINT` (`UnitCodeId::to_smallint`, `UnitCode::from_smallint`). `UnitCode::from_id` returns `None` for an identity a newer release assigned. `UnitCode::ALL` lists the codes in identity order, and `ALL[id - 1]` is the code with identity `id`. The registry lives in `cefact-units/data/ids.tsv`; `cefact-units/data/ids-frozen.txt` records, for each published release, how many registry lines it covered and their SHA-256, and a test fails if a published prefix ever changes.

## Usage

```toml
[dependencies]
cefact-units = "0.2"
```

```rust
use cefact_units::{Status, UnitCode, UnknownCode};

fn main() -> Result<(), UnknownCode> {
    let kg: UnitCode = "KGM".parse()?;
    assert_eq!(kg.info().name, "kilogram");
    assert_eq!(kg.info().symbol, Some("kg"));
    assert_eq!(kg.status(), Status::Current);

    // Store the identity in a SMALLINT column and read it back.
    let stored: i16 = kg.id().to_smallint();
    assert_eq!(UnitCode::from_smallint(stored), Some(kg));

    // Parsing is case-sensitive; ASCII case folding is opt-in per call.
    assert!("kgm".parse::<UnitCode>().is_err());
    assert_eq!(UnitCode::from_code_ignore_case("kgm"), Some(kg));

    for code in UnitCode::ALL {
        println!("{} {code}: {}", code.id().get(), code.info().name);
    }
    Ok(())
}
```

## Features

- `serde` — Serialize `UnitCode` as its common code and deserialize it from any string or byte string, borrowed or not; serialize `UnitInfo`; serialize and deserialize `Status` (kebab-case) and `Revision`

## Minimum Supported Rust Version

Rust 1.88 or later (edition 2024).

## Data Attribution

Unit code data is derived from [UN/CEFACT Recommendation No. 20](https://unece.org/trade/uncefact/cl-recommendations) maintained by the United Nations Economic Commission for Europe (UNECE). The data is provided for public use under UNECE's open data policy.

## License

MIT
