# cefact-units

UN/CEFACT Recommendation No. 20 — Codes for Units of Measure Used in International Trade (Revision 17, 2021).

All data is generated at compile time from `rec20.xlsx` and embedded as `static` data. No runtime allocations, no I/O.

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

## License

MIT
