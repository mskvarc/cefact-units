//! Looks up a few common codes and lists the first codes by identity.

use cefact_units::UnitCode;
use cefact_units::UnknownCode;

fn main() -> Result<(), UnknownCode> {
    let kg: UnitCode = "KGM".parse()?;
    let info = kg.info();
    println!(
        "{kg}: {} ({:?}), measures {:?}",
        info.name, info.symbol, info.quantity
    );
    println!("identity {}, status {:?}", kg.id().get(), kg.status());

    let litre = UnitCode::from_code_ignore_case("ltr").ok_or(UnknownCode)?;
    println!("{litre}: {}", litre.info().name);

    println!(
        "{} codes, highest identity {}",
        UnitCode::ALL.len(),
        UnitCode::MAX_ID.get()
    );
    for code in UnitCode::ALL.iter().take(10) {
        println!("  {:>4} {code}: {}", code.id().get(), code.info().name);
    }
    Ok(())
}
