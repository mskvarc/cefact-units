//! Status markers a published workbook misprints, each with the marker it stands for.
//!
//! The list is closed: any other marker holding an undefined flag fails generation, so a new misprint is read only
//! once it is listed here.

/// One misprinted marker of one code in one revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarkerErratum {
    /// Revision of Recommendation No. 20 printing the marker.
    revision: u16,
    /// Common code the marker belongs to.
    code: &'static str,
    /// Marker as printed.
    printed: &'static str,
    /// Marker the row stands for.
    intended: &'static str,
}

/// Every known misprint.
const ERRATA: &[MarkerErratum] = &[
    // `I` for the changed-entry flag `|` the same revision prints as `D|` on other rows; the code is deprecated
    // in every later revision.
    MarkerErratum {
        revision: 4,
        code: "78",
        printed: "DI",
        intended: "D|",
    },
    // `1` for the changed-entry flag `|`: the code's Annex II row in the same revision reads `#|`.
    MarkerErratum {
        revision: 6,
        code: "E41",
        printed: "#1",
        intended: "#|",
    },
    // U+00D7 MULTIPLICATION SIGN for `X`: the code is deleted in every revision, and its Annex II row reads `X`.
    MarkerErratum {
        revision: 11,
        code: "KTM",
        printed: "\u{D7}",
        intended: "X",
    },
    MarkerErratum {
        revision: 12,
        code: "KTM",
        printed: "\u{D7}",
        intended: "X",
    },
];

/// The marker the row of `code` in `revision` stands for: the listed correction of a known misprint, else `printed`.
pub fn intended_marker<'m>(revision: u16, code: &str, printed: &'m str) -> &'m str {
    ERRATA
        .iter()
        .find(|erratum| {
            erratum.revision == revision && erratum.code == code && erratum.printed == printed
        })
        .map_or(printed, |erratum| erratum.intended)
}

#[cfg(test)]
mod tests {
    use crate::marker_erratum::ERRATA;
    use crate::marker_erratum::intended_marker;
    use crate::status::Status;

    #[test]
    fn a_listed_misprint_reads_as_its_intended_marker() {
        assert_eq!(intended_marker(4, "78", "DI"), "D|");
        assert_eq!(intended_marker(6, "E41", "#1"), "#|");
        assert_eq!(intended_marker(11, "KTM", "\u{D7}"), "X");
        assert_eq!(intended_marker(12, "KTM", "\u{D7}"), "X");
    }

    #[test]
    fn a_misprint_is_corrected_only_for_its_revision_and_code() {
        assert_eq!(intended_marker(13, "KTM", "\u{D7}"), "\u{D7}");
        assert_eq!(intended_marker(11, "KGM", "\u{D7}"), "\u{D7}");
        assert_eq!(intended_marker(4, "78", "D"), "D");
    }

    #[test]
    fn every_misprint_is_undefined_and_every_correction_is_defined() {
        for erratum in ERRATA {
            assert_eq!(Status::from_marker(erratum.printed), None, "{erratum:?}");
            assert!(
                Status::from_marker(erratum.intended).is_some(),
                "{erratum:?}"
            );
        }
    }
}
