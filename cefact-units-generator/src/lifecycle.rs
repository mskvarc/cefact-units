//! The status history of every Recommendation No. 20 code across the revisions the sources publish.
//!
//! Within one revision a code listed on several rows takes its most current status. Across revisions the history
//! records the status at the code's first listing and every revision where it changes. Once listed, a code is listed
//! in every later revision: the recommendation withdraws a code by marking it `X`, never by dropping its row, and a
//! source that drops one fails generation.

use crate::generation_error::GenerationError;
use crate::status::Status;
use crate::workbook_row::WorkbookRow;
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

/// The rows one revision publishes.
#[derive(Debug)]
pub struct RevisionRows {
    /// Revision of Recommendation No. 20.
    pub revision: u16,
    /// Every code row of the revision.
    pub rows: Vec<WorkbookRow>,
}

impl Lifecycle {
    /// A code listed in one status from `first_listed` on.
    pub const fn unchanged(first_listed: StatusChange) -> Lifecycle {
        Lifecycle {
            first_listed,
            changes: Vec::new(),
        }
    }

    /// Extends the history with the status `change` gives, when it differs from the latest one.
    pub fn record(&mut self, change: StatusChange) {
        if self.latest_status() != change.status {
            self.changes.push(change);
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

    /// Traces every code through `revisions`, given in ascending revision order. A code a revision no longer lists
    /// after an earlier one did fails, naming the code and the revision.
    pub fn trace(
        revisions: &[RevisionRows],
    ) -> Result<BTreeMap<String, Lifecycle>, GenerationError> {
        revisions.iter().try_fold(
            BTreeMap::new(),
            |mut lifecycles: BTreeMap<String, Lifecycle>, revision| {
                let statuses = statuses_in(&revision.rows);
                if let Some(dropped) = lifecycles
                    .keys()
                    .find(|code| !statuses.contains_key(code.as_str()))
                {
                    return Err(GenerationError::DroppedCode {
                        code: dropped.clone(),
                        revision: revision.revision,
                    });
                }
                for (code, status) in statuses {
                    let change = StatusChange {
                        revision: revision.revision,
                        status,
                    };
                    match lifecycles.entry(code.to_owned()) {
                        Entry::Vacant(first) => {
                            first.insert(Lifecycle::unchanged(change));
                        }
                        Entry::Occupied(mut listed) => listed.get_mut().record(change),
                    }
                }
                Ok(lifecycles)
            },
        )
    }
}

/// Every code of one revision with its most current status among the rows listing it.
fn statuses_in(rows: &[WorkbookRow]) -> BTreeMap<&str, Status> {
    rows.iter().fold(BTreeMap::new(), |mut statuses, row| {
        statuses
            .entry(row.code.as_str())
            .and_modify(|status: &mut Status| *status = (*status).min(row.status))
            .or_insert(row.status);
        statuses
    })
}

#[cfg(test)]
mod tests {
    use crate::annex::Annex;
    use crate::generation_error::GenerationError;
    use crate::lifecycle::Lifecycle;
    use crate::lifecycle::RevisionRows;
    use crate::lifecycle::StatusChange;
    use crate::status::Status;
    use crate::workbook_row::WorkbookRow;

    fn row(code: &str, status: Status) -> WorkbookRow {
        WorkbookRow {
            annex: Annex::TwoAndThree,
            row: 2,
            status,
            code: code.to_owned(),
            name: code.to_owned(),
            symbol: None,
            quantity: None,
            sector: None,
            conversion_factor: None,
            level_category: None,
            description: None,
        }
    }

    fn revision(revision: u16, rows: &[(&str, Status)]) -> RevisionRows {
        RevisionRows {
            revision,
            rows: rows
                .iter()
                .map(|(code, status)| row(code, *status))
                .collect(),
        }
    }

    const fn change(revision: u16, status: Status) -> StatusChange {
        StatusChange { revision, status }
    }

    fn history_of(revisions: &[RevisionRows], code: &str) -> Option<Vec<StatusChange>> {
        Lifecycle::trace(revisions).ok().and_then(|lifecycles| {
            lifecycles
                .get(code)
                .map(|lifecycle| lifecycle.history().collect())
        })
    }

    #[test]
    fn a_code_unchanged_since_its_first_listing_has_one_entry() {
        let revisions = [
            revision(4, &[("KGM", Status::Current)]),
            revision(5, &[("KGM", Status::Current)]),
        ];
        assert_eq!(
            history_of(&revisions, "KGM"),
            Some(vec![change(4, Status::Current)])
        );
    }

    #[test]
    fn a_code_first_listed_in_a_later_revision_starts_there() {
        let revisions = [
            revision(4, &[("KGM", Status::Current)]),
            revision(5, &[("KGM", Status::Current), ("MNJ", Status::Current)]),
        ];
        assert_eq!(
            history_of(&revisions, "MNJ"),
            Some(vec![change(5, Status::Current)])
        );
    }

    #[test]
    fn every_status_change_is_recorded_at_its_revision() {
        let revisions = [
            revision(4, &[("R4", Status::Deprecated)]),
            revision(5, &[("R4", Status::Deprecated)]),
            revision(6, &[("R4", Status::Deleted)]),
        ];
        assert_eq!(
            history_of(&revisions, "R4"),
            Some(vec![
                change(4, Status::Deprecated),
                change(6, Status::Deleted)
            ])
        );
    }

    #[test]
    fn a_reinstated_code_returns_to_current() {
        let revisions = [
            revision(4, &[("HH", Status::Deleted)]),
            revision(5, &[("HH", Status::Current)]),
        ];
        assert_eq!(
            history_of(&revisions, "HH"),
            Some(vec![change(4, Status::Deleted), change(5, Status::Current)])
        );
    }

    #[test]
    fn a_code_on_several_rows_of_a_revision_takes_its_most_current_status() {
        let revisions = [revision(
            17,
            &[
                ("A91", Status::Deprecated),
                ("A91", Status::Current),
                ("BB", Status::Deleted),
                ("BB", Status::Deprecated),
            ],
        )];
        assert_eq!(
            history_of(&revisions, "A91"),
            Some(vec![change(17, Status::Current)])
        );
        assert_eq!(
            history_of(&revisions, "BB"),
            Some(vec![change(17, Status::Deprecated)])
        );
    }

    #[test]
    fn a_code_a_later_revision_drops_fails_generation() {
        let revisions = [
            revision(4, &[("AA", Status::Current), ("BB", Status::Current)]),
            revision(5, &[("BB", Status::Current)]),
        ];
        assert!(matches!(
            Lifecycle::trace(&revisions),
            Err(GenerationError::DroppedCode { ref code, revision: 5 }) if code == "AA"
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
