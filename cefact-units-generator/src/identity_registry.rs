//! The append-only registry giving every common code a stable numeric identity.
//!
//! The registry is one `<identity>\t<code>` line per code, identities dense from 1 in line order. A code keeps its
//! identity forever: codes the sources newly list are appended, and a registered code is never removed or
//! renumbered, so a value stored under an identity decodes to the same code in every later release.

use crate::catalogue::UnitRecord;
use crate::generation_error::GenerationError;
use crate::workbook_row::is_common_code_shape;
use std::collections::HashMap;
use std::collections::HashSet;

/// Identity of a code, from 1 to [`Identity::MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity(u16);

impl Identity {
    /// Highest identity: `i16::MAX`, so every identity fits a signed 16-bit SQL `SMALLINT`.
    pub const MAX: u16 = 32_767;

    /// Identity of the code on the zero-based registry line `index`, `None` past [`Identity::MAX`].
    fn at_line(index: usize) -> Option<Identity> {
        index
            .checked_add(1)
            .and_then(|position| u16::try_from(position).ok())
            .filter(|identity| *identity <= Identity::MAX)
            .map(Identity)
    }

    /// The identity as a number.
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// One registered code.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryEntry {
    /// Identity, equal to the one-based line number.
    identity: Identity,
    /// Common code.
    code: String,
}

/// Every registered code in identity order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRegistry {
    /// Entries, the entry at index `i` holding identity `i + 1`.
    entries: Vec<RegistryEntry>,
}

/// A code record with its identity.
#[derive(Debug, Clone, Copy)]
pub struct IdentifiedUnit<'u> {
    /// Identity of the code.
    pub identity: Identity,
    /// The code's record.
    pub unit: &'u UnitRecord,
}

/// The registry extended with every newly listed code, and the records in identity order.
#[derive(Debug)]
pub struct Assignment<'u> {
    /// Registry covering exactly the listed codes.
    pub registry: IdentityRegistry,
    /// Every record, in identity order.
    pub units: Vec<IdentifiedUnit<'u>>,
}

impl IdentityRegistry {
    /// Reads the registry text. A malformed line, an identity other than its line number, a malformed code, a code
    /// registered twice and an identity past [`Identity::MAX`] fail.
    pub fn parse(text: &str) -> Result<IdentityRegistry, GenerationError> {
        let mut seen: HashSet<&str> = HashSet::new();
        let entries = text
            .lines()
            .enumerate()
            .map(|(index, line)| {
                let line_number = index + 1;
                let malformed = || GenerationError::MalformedRegistryLine {
                    line: line_number,
                    text: line.to_owned(),
                };
                let (identity, code) = line.split_once('\t').ok_or_else(malformed)?;
                let identity: u16 = identity.parse().map_err(|_| malformed())?;
                if !is_common_code_shape(code) {
                    return Err(malformed());
                }
                let expected =
                    Identity::at_line(index).ok_or_else(|| GenerationError::IdentityOverflow {
                        code: code.to_owned(),
                    })?;
                if identity != expected.get() {
                    return Err(GenerationError::IdentityOutOfSequence {
                        line: line_number,
                        found: identity,
                    });
                }
                if !seen.insert(code) {
                    return Err(GenerationError::DuplicateRegisteredCode {
                        code: code.to_owned(),
                    });
                }
                Ok(RegistryEntry {
                    identity: expected,
                    code: code.to_owned(),
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(IdentityRegistry { entries })
    }

    /// Gives every record an identity: a registered code keeps its own, and the codes the registry lacks are
    /// appended in ascending code order. A registered code absent from `units` fails, since codes are never removed.
    pub fn assign(self, units: &[UnitRecord]) -> Result<Assignment<'_>, GenerationError> {
        let mut unregistered: HashMap<&str, &UnitRecord> = units
            .iter()
            .map(|unit| (unit.code.as_str(), unit))
            .collect();
        let mut identified = self
            .entries
            .iter()
            .map(|entry| {
                unregistered
                    .remove(entry.code.as_str())
                    .map(|unit| IdentifiedUnit {
                        identity: entry.identity,
                        unit,
                    })
                    .ok_or_else(|| GenerationError::UnlistedRegisteredCode {
                        code: entry.code.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut appended: Vec<&UnitRecord> = unregistered.into_values().collect();
        appended.sort_by(|left, right| left.code.cmp(&right.code));
        let mut entries = self.entries;
        for unit in appended {
            let identity = Identity::at_line(entries.len()).ok_or_else(|| {
                GenerationError::IdentityOverflow {
                    code: unit.code.clone(),
                }
            })?;
            // The registry owns its codes: it outlives the records it was assigned against.
            entries.push(RegistryEntry {
                identity,
                code: unit.code.clone(),
            });
            identified.push(IdentifiedUnit { identity, unit });
        }
        Ok(Assignment {
            registry: IdentityRegistry { entries },
            units: identified,
        })
    }

    /// The registry text, one `<identity>\t<code>` line per code.
    pub fn render(&self) -> String {
        self.entries
            .iter()
            .map(|entry| format!("{}\t{}\n", entry.identity.get(), entry.code))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::catalogue::UnitRecord;
    use crate::code_list::CodeList;
    use crate::generation_error::GenerationError;
    use crate::identity_registry::Assignment;
    use crate::identity_registry::Identity;
    use crate::identity_registry::IdentityRegistry;
    use crate::lifecycle::Lifecycle;
    use crate::lifecycle::StatusChange;
    use crate::status::Status;

    fn unit(code: &str) -> UnitRecord {
        UnitRecord {
            code: code.to_owned(),
            code_variant: code.to_owned(),
            name: code.to_owned(),
            symbol: None,
            quantity: None,
            sector: None,
            conversion_factor: None,
            level_category: None,
            description: None,
            source: CodeList::Recommendation20,
            lifecycle: Lifecycle::unchanged(StatusChange {
                revision: 17,
                status: Status::Current,
            }),
        }
    }

    fn assign<'u>(
        registry: &str,
        units: &'u [UnitRecord],
    ) -> Result<Assignment<'u>, GenerationError> {
        IdentityRegistry::parse(registry)?.assign(units)
    }

    fn identities(assignment: &Assignment<'_>) -> Vec<(u16, String)> {
        assignment
            .units
            .iter()
            .map(|identified| (identified.identity.get(), identified.unit.code.clone()))
            .collect()
    }

    /// `count` distinct three-character codes, `000`, `001`, ... in base 36.
    fn three_character_codes(count: usize) -> impl Iterator<Item = String> {
        const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        (0..count).map(|number| {
            [number / (36 * 36), number / 36 % 36, number % 36]
                .iter()
                .filter_map(|digit| DIGITS.get(*digit).copied().map(char::from))
                .collect()
        })
    }

    #[test]
    fn new_codes_are_appended_in_ascending_order() {
        let units = [unit("CC"), unit("AA"), unit("BB")];
        let assignment = assign("1\tBB\n", &units);
        assert_eq!(
            assignment.as_ref().map(identities).ok(),
            Some(vec![
                (1, "BB".to_owned()),
                (2, "AA".to_owned()),
                (3, "CC".to_owned())
            ])
        );
        assert_eq!(
            assignment
                .map(|assignment| assignment.registry.render())
                .ok()
                .as_deref(),
            Some("1\tBB\n2\tAA\n3\tCC\n")
        );
    }

    #[test]
    fn an_assigned_code_is_never_renumbered() {
        let units = [unit("AA"), unit("ZZ")];
        let assignment = assign("1\tZZ\n2\tAA\n", &units);
        assert_eq!(
            assignment.as_ref().map(identities).ok(),
            Some(vec![(1, "ZZ".to_owned()), (2, "AA".to_owned())])
        );
    }

    #[test]
    fn a_code_no_source_lists_fails_generation() {
        let units = [unit("BB")];
        let assignment = assign("1\tAA\n", &units);
        assert!(matches!(
            assignment,
            Err(GenerationError::UnlistedRegisteredCode { ref code }) if code == "AA"
        ));
    }

    #[test]
    fn an_identity_beyond_the_smallint_range_fails_generation() {
        let registry: String = three_character_codes(usize::from(Identity::MAX))
            .zip(1..)
            .map(|(code, identity)| format!("{identity}\t{code}\n"))
            .collect();
        let mut units: Vec<UnitRecord> = three_character_codes(usize::from(Identity::MAX))
            .map(|code| unit(&code))
            .collect();
        units.push(unit("ZZ"));
        let assignment = assign(&registry, &units);
        assert!(matches!(
            assignment,
            Err(GenerationError::IdentityOverflow { ref code }) if code == "ZZ"
        ));
    }

    #[test]
    fn a_registry_filled_to_the_highest_identity_is_accepted() {
        let registry: String = three_character_codes(usize::from(Identity::MAX))
            .zip(1..)
            .map(|(code, identity)| format!("{identity}\t{code}\n"))
            .collect();
        let parsed = IdentityRegistry::parse(&registry);
        assert_eq!(parsed.map(|parsed| parsed.render()).ok(), Some(registry));
    }

    #[test]
    fn an_identity_other_than_its_line_number_fails() {
        let parsed = IdentityRegistry::parse("1\tAA\n3\tBB\n");
        assert!(matches!(
            parsed,
            Err(GenerationError::IdentityOutOfSequence { line: 2, found: 3 })
        ));
    }

    #[test]
    fn a_code_registered_twice_fails() {
        let parsed = IdentityRegistry::parse("1\tAA\n2\tAA\n");
        assert!(matches!(
            parsed,
            Err(GenerationError::DuplicateRegisteredCode { ref code }) if code == "AA"
        ));
    }

    #[test]
    fn a_malformed_registry_line_fails_naming_the_line() {
        for text in ["1 AA", "x\tAA", "1\tkgm", "1\t", "\tAA", "-1\tAA"] {
            let parsed = IdentityRegistry::parse(text);
            assert!(
                matches!(
                    parsed,
                    Err(GenerationError::MalformedRegistryLine { line: 1, .. })
                ),
                "{text:?}"
            );
        }
    }

    #[test]
    fn an_empty_registry_numbers_every_code_from_one() {
        let units = [unit("BB"), unit("AA")];
        let assignment = assign("", &units);
        assert_eq!(
            assignment.as_ref().map(identities).ok(),
            Some(vec![(1, "AA".to_owned()), (2, "BB".to_owned())])
        );
    }
}
