//! The status a Recommendation No. 20 row gives its code, read from the row's status marker.

/// Status of a code, ordered from most to least current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// Listed for use: no marker, `+` (added), `#` (name changed), `¦` (changed entry) or `=` (reinstated).
    Current,
    /// Marked `D`: listed but not to be used for new data.
    Deprecated,
    /// Marked `X`: removed from the recommendation.
    Deleted,
}

impl Status {
    /// Maps a trimmed status marker to its status, `None` for a marker the recommendation does not define.
    pub fn from_marker(marker: &str) -> Option<Status> {
        match marker {
            "" | "+" | "#" | "¦" | "=" => Some(Status::Current),
            "D" => Some(Status::Deprecated),
            "X" => Some(Status::Deleted),
            _ => None,
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
    fn statuses_order_from_most_to_least_current() {
        assert!(Status::Current < Status::Deprecated);
        assert!(Status::Deprecated < Status::Deleted);
    }
}
