//! The status history of every code across the revisions of Recommendation No. 20 the sources publish.
//!
//! Within one source a code listed on several rows takes its most current status. Across sources the history records
//! the status at the code's first listing and every Recommendation No. 20 revision where it changes; two sources
//! listed from the same revision count as one, the later one prevailing. Once listed, a code is listed by every later
//! source: the recommendations withdraw a code by marking it `X`, never by dropping its row, and a source that drops
//! one fails generation.

use crate::generation_error::GenerationError;
use crate::status::Status;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

/// The status a code takes from one revision on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusChange {
    /// Revision of Recommendation No. 20.
    pub revision: u16,
    /// Status from that revision on.
    pub status: Status,
}

/// A code's status history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lifecycle {
    /// Status in the first revision listing the code.
    pub first_listed: StatusChange,
    /// Every later change, in ascending revision order, each to a status other than the one before it.
    pub changes: Vec<StatusChange>,
}

/// Every code one source lists, with its most current status among the rows listing it.
#[derive(Debug)]
pub struct Listing<'r> {
    /// Path of the source, for error reports.
    pub source: &'static str,
    /// Revision of Recommendation No. 20 the source's codes are listed from.
    pub revision: u16,
    /// Status of each code.
    pub statuses: BTreeMap<&'r str, Status>,
}

impl<'r> Listing<'r> {
    /// The listing of `rows`, read by `listing` as code and status, published by `source` and listed from `revision`.
    pub fn of<R>(
        source: &'static str,
        revision: u16,
        rows: &'r [R],
        listing: impl Fn(&'r R) -> (&'r str, Status),
    ) -> Listing<'r> {
        let statuses = rows.iter().map(listing).fold(
            BTreeMap::new(),
            |mut statuses: BTreeMap<&str, Status>, (code, status)| {
                statuses
                    .entry(code)
                    .and_modify(|listed| *listed = (*listed).min(status))
                    .or_insert(status);
                statuses
            },
        );
        Listing {
            source,
            revision,
            statuses,
        }
    }
}

impl Lifecycle {
    /// A code listed in one status from `first_listed` on.
    pub const fn unchanged(first_listed: StatusChange) -> Lifecycle {
        Lifecycle {
            first_listed,
            changes: Vec::new(),
        }
    }

    /// Extends the history with `change`. A change at the revision of the latest entry replaces that entry's status,
    /// and an entry left with the status before it is dropped.
    pub fn record(&mut self, change: StatusChange) {
        match self.changes.last_mut() {
            Some(last) if last.revision == change.revision => {
                last.status = change.status;
                let before = self
                    .changes
                    .iter()
                    .rev()
                    .nth(1)
                    .map_or(self.first_listed.status, |previous| previous.status);
                if before == change.status {
                    self.changes.pop();
                }
            }
            None if self.first_listed.revision == change.revision => {
                self.first_listed.status = change.status;
            }
            Some(_) | None => {
                if self.latest_status() != change.status {
                    self.changes.push(change);
                }
            }
        }
    }

    /// Status in the latest revision.
    pub fn latest_status(&self) -> Status {
        self.changes
            .last()
            .map_or(self.first_listed.status, |change| change.status)
    }

    /// The first listing followed by every change.
    pub fn history(&self) -> impl Iterator<Item = StatusChange> {
        std::iter::once(self.first_listed).chain(self.changes.iter().copied())
    }

    /// Traces every code through `listings`, given in ascending revision order. A code a listing no longer holds after
    /// an earlier one did fails, naming the code and the source.
    pub fn trace(listings: &[Listing<'_>]) -> Result<BTreeMap<String, Lifecycle>, GenerationError> {
        listings
            .iter()
            .try_fold(BTreeMap::new(), |mut lifecycles, listing| {
                check_none_dropped(&lifecycles, listing)?;
                for (code, status) in &listing.statuses {
                    let change = StatusChange {
                        revision: listing.revision,
                        status: *status,
                    };
                    match lifecycles.entry((*code).to_owned()) {
                        Entry::Vacant(first) => {
                            first.insert(Lifecycle::unchanged(change));
                        }
                        Entry::Occupied(mut listed) => listed.get_mut().record(change),
                    }
                }
                Ok(lifecycles)
            })
    }

    /// Pairs each of `units`, the merged codes of the latest source `source` listed from `revision`, with its history
    /// across `earlier` and that source. `listing` reads a unit's code and status.
    pub fn follow<U>(
        earlier: &[Listing<'_>],
        source: &'static str,
        revision: u16,
        units: Vec<U>,
        listing: impl Fn(&U) -> (&str, Status),
    ) -> Result<Vec<(U, Lifecycle)>, GenerationError> {
        let mut traced = Lifecycle::trace(earlier)?;
        check_none_dropped(&traced, &Listing::of(source, revision, &units, &listing))?;
        Ok(units
            .into_iter()
            .map(|unit| {
                let (code, status) = listing(&unit);
                let change = StatusChange { revision, status };
                let lifecycle = match traced.remove(code) {
                    Some(mut earlier) => {
                        earlier.record(change);
                        earlier
                    }
                    None => Lifecycle::unchanged(change),
                };
                (unit, lifecycle)
            })
            .collect())
    }
}

/// Fails when `listing` no longer holds a code `lifecycles` traced.
fn check_none_dropped(
    lifecycles: &BTreeMap<String, Lifecycle>,
    listing: &Listing<'_>,
) -> Result<(), GenerationError> {
    match lifecycles
        .keys()
        .find(|code| !listing.statuses.contains_key(code.as_str()))
    {
        Some(dropped) => Err(GenerationError::DroppedCode {
            code: dropped.clone(),
            path: listing.source,
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::generation_error::GenerationError;
    use crate::lifecycle::Lifecycle;
    use crate::lifecycle::Listing;
    use crate::lifecycle::StatusChange;
    use crate::status::Status;

    const SOURCE: &str = "sources/fixture.xls";

    fn listing<'r>(revision: u16, rows: &'r [(&'r str, Status)]) -> Listing<'r> {
        Listing::of(SOURCE, revision, rows, |(code, status)| (*code, *status))
    }

    const fn change(revision: u16, status: Status) -> StatusChange {
        StatusChange { revision, status }
    }

    fn history_of(listings: &[Listing<'_>], code: &str) -> Option<Vec<StatusChange>> {
        Lifecycle::trace(listings).ok().and_then(|lifecycles| {
            lifecycles
                .get(code)
                .map(|lifecycle| lifecycle.history().collect())
        })
    }

    #[test]
    fn a_code_unchanged_since_its_first_listing_has_one_entry() {
        let listings = [
            listing(4, &[("KGM", Status::Current)]),
            listing(5, &[("KGM", Status::Current)]),
        ];
        assert_eq!(
            history_of(&listings, "KGM"),
            Some(vec![change(4, Status::Current)])
        );
    }

    #[test]
    fn a_code_first_listed_in_a_later_revision_starts_there() {
        let listings = [
            listing(4, &[("KGM", Status::Current)]),
            listing(5, &[("KGM", Status::Current), ("MNJ", Status::Current)]),
        ];
        assert_eq!(
            history_of(&listings, "MNJ"),
            Some(vec![change(5, Status::Current)])
        );
    }

    #[test]
    fn every_status_change_is_recorded_at_its_revision() {
        let listings = [
            listing(4, &[("R4", Status::Deprecated)]),
            listing(5, &[("R4", Status::Deprecated)]),
            listing(6, &[("R4", Status::Deleted)]),
        ];
        assert_eq!(
            history_of(&listings, "R4"),
            Some(vec![
                change(4, Status::Deprecated),
                change(6, Status::Deleted)
            ])
        );
    }

    #[test]
    fn a_reinstated_code_returns_to_current() {
        let listings = [
            listing(4, &[("HH", Status::Deleted)]),
            listing(5, &[("HH", Status::Current)]),
        ];
        assert_eq!(
            history_of(&listings, "HH"),
            Some(vec![change(4, Status::Deleted), change(5, Status::Current)])
        );
    }

    #[test]
    fn a_code_on_several_rows_of_a_source_takes_its_most_current_status() {
        let listings = [listing(
            17,
            &[
                ("A91", Status::Deprecated),
                ("A91", Status::Current),
                ("BB", Status::Deleted),
                ("BB", Status::Deprecated),
            ],
        )];
        assert_eq!(
            history_of(&listings, "A91"),
            Some(vec![change(17, Status::Current)])
        );
        assert_eq!(
            history_of(&listings, "BB"),
            Some(vec![change(17, Status::Deprecated)])
        );
    }

    #[test]
    fn of_two_sources_listed_from_one_revision_the_later_prevails() {
        let listings = [
            listing(4, &[("AA", Status::Current), ("BB", Status::Current)]),
            listing(6, &[("AA", Status::Deprecated), ("BB", Status::Deprecated)]),
            listing(
                6,
                &[
                    ("AA", Status::Deleted),
                    ("BB", Status::Current),
                    ("CC", Status::Current),
                ],
            ),
        ];
        assert_eq!(
            history_of(&listings, "AA"),
            Some(vec![change(4, Status::Current), change(6, Status::Deleted)])
        );
        assert_eq!(
            history_of(&listings, "BB"),
            Some(vec![change(4, Status::Current)])
        );
        assert_eq!(
            history_of(&listings, "CC"),
            Some(vec![change(6, Status::Current)])
        );
    }

    #[test]
    fn a_later_source_from_the_first_revision_replaces_the_first_status() {
        let listings = [
            listing(4, &[("AA", Status::Current)]),
            listing(4, &[("AA", Status::Deleted)]),
        ];
        assert_eq!(
            history_of(&listings, "AA"),
            Some(vec![change(4, Status::Deleted)])
        );
    }

    #[test]
    fn a_code_a_later_source_drops_fails_generation() {
        let listings = [
            listing(4, &[("AA", Status::Current), ("BB", Status::Current)]),
            listing(5, &[("BB", Status::Current)]),
        ];
        assert!(matches!(
            Lifecycle::trace(&listings),
            Err(GenerationError::DroppedCode { ref code, path: SOURCE }) if code == "AA"
        ));
    }

    #[test]
    fn following_pairs_each_latest_unit_with_its_history() {
        let earlier = [listing(4, &[("AA", Status::Current)])];
        let latest = vec![("AA", Status::Deprecated), ("BB", Status::Current)];
        let followed = Lifecycle::follow(&earlier, SOURCE, 9, latest, |(code, status)| {
            (*code, *status)
        });
        let histories: Vec<_> = followed
            .iter()
            .flatten()
            .map(|((code, _), lifecycle)| (*code, lifecycle.history().collect::<Vec<_>>()))
            .collect();
        assert_eq!(
            histories,
            [
                (
                    "AA",
                    vec![change(4, Status::Current), change(9, Status::Deprecated)]
                ),
                ("BB", vec![change(9, Status::Current)]),
            ]
        );
    }

    #[test]
    fn following_fails_when_the_latest_listing_drops_a_code() {
        let earlier = [listing(4, &[("AA", Status::Current)])];
        let latest = vec![("BB", Status::Current)];
        assert!(matches!(
            Lifecycle::follow(&earlier, SOURCE, 9, latest, |(code, status)| (*code, *status)),
            Err(GenerationError::DroppedCode { ref code, .. }) if code == "AA"
        ));
    }

    #[test]
    fn the_latest_status_is_the_last_change() {
        let lifecycle = Lifecycle {
            first_listed: change(4, Status::Deleted),
            changes: vec![change(6, Status::Current)],
        };
        assert_eq!(lifecycle.latest_status(), Status::Current);
        assert_eq!(
            Lifecycle::unchanged(change(4, Status::Deprecated)).latest_status(),
            Status::Deprecated
        );
    }
}
