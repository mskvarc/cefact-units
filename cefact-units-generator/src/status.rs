//! The status a code row gives its code, read from the row's status marker.

/// Status of a code, ordered from most to least current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// Listed for use: no marker, `+` (added), `#` (name changed), `¦` or `|` (changed entry) or `=` (reinstated).
    Current,
    /// Marked `D`: listed but not to be used for new data.
    Deprecated,
    /// Marked `X`: removed from the recommendation.
    Deleted,
}

impl Status {
    /// Maps a trimmed status marker to its status, `None` for a marker holding a flag the recommendation does not
    /// define.
    ///
    /// A marker is a set of flags, and the revisions up to 7 combine them (`D|` deprecated and changed, `+D` added and
    /// deprecated, `X|` deleted and changed). `X` outranks `D`, which outranks every other flag. The revisions up to 7
    /// print the changed-entry flag as `|`, the later ones as `¦`.
    pub fn from_marker(marker: &str) -> Option<Status> {
        marker
            .chars()
            .try_fold(Status::Current, |status, flag| match flag {
                '+' | '#' | '¦' | '|' | '=' => Some(status),
                'D' => Some(status.max(Status::Deprecated)),
                'X' => Some(Status::Deleted),
                _ => None,
            })
    }

    /// Path of the library `Status` variant.
    pub const fn library_path(self) -> &'static str {
        match self {
            Status::Current => "Status::Current",
            Status::Deprecated => "Status::Deprecated",
            Status::Deleted => "Status::Deleted",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::status::Status;

    #[test]
    fn status_markers_map_to_statuses() {
        let expected = [
            ("", Some(Status::Current)),
            ("+", Some(Status::Current)),
            ("#", Some(Status::Current)),
            ("¦", Some(Status::Current)),
            ("|", Some(Status::Current)),
            ("=", Some(Status::Current)),
            ("D", Some(Status::Deprecated)),
            ("X", Some(Status::Deleted)),
            ("?", None),
            ("d", None),
            ("x", None),
        ];
        for (marker, status) in expected {
            assert_eq!(Status::from_marker(marker), status, "marker {marker:?}");
        }
    }

    #[test]
    fn a_compound_marker_takes_its_least_current_flag() {
        let expected = [
            ("#|", Some(Status::Current)),
            ("D|", Some(Status::Deprecated)),
            ("+D", Some(Status::Deprecated)),
            ("D#", Some(Status::Deprecated)),
            ("X|", Some(Status::Deleted)),
            ("X#", Some(Status::Deleted)),
            ("XD", Some(Status::Deleted)),
            ("DI", None),
            ("#1", None),
            ("\u{D7}", None),
        ];
        for (marker, status) in expected {
            assert_eq!(Status::from_marker(marker), status, "marker {marker:?}");
        }
    }

    #[test]
    fn statuses_order_from_most_to_least_current() {
        assert!(Status::Current < Status::Deprecated);
        assert!(Status::Deprecated < Status::Deleted);
    }
}
