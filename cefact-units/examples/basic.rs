//! Basic usage of cefact-units.

use cefact_units::{UnitCode, UnitOfMeasure};

fn main() {
    // Look up by code
    let kg = UnitOfMeasure::from_code("KGM").unwrap();
    println!("Code: {}", kg.code());
    println!("Name: {}", kg.name());
    println!("Symbol: {:?}", kg.symbol());
    println!("Quantity: {:?}", kg.quantity());
    println!();

    // Parse from string
    let meter: UnitOfMeasure = "MTR".parse().unwrap();
    println!("{} = {}", meter.code(), meter.name());
    println!();

    // Use strongly-typed UnitCode
    let code: UnitCode = "LTR".parse().unwrap();
    let unit = UnitOfMeasure::from_unit_code(code);
    println!("{} -> {}", code, unit.name());
    println!();

    // Iterate all units
    println!("Total units: {}", UnitOfMeasure::ALL.len());
    println!("\nFirst 10 units:");
    for unit in UnitOfMeasure::ALL.iter().take(10) {
        println!("  {} — {}", unit.code(), unit.name());
    }
}
