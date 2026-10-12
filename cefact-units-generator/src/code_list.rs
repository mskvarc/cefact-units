//! The UN/CEFACT recommendation a source workbook publishes and a code is drawn from.

/// A recommendation publishing a code list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CodeList {
    /// Recommendation No. 20, Codes for Units of Measure Used in International Trade.
    Recommendation20,
    /// Recommendation No. 21, Codes for Passengers, Types of Cargo, Packages and Packaging Materials. Its package type
    /// codes are units of count once prefixed with `X`, per Recommendation No. 20 introduction note 2.
    Recommendation21,
}

impl CodeList {
    /// Path of the library `CodeList` variant.
    pub const fn library_path(self) -> &'static str {
        match self {
            CodeList::Recommendation20 => "CodeList::Recommendation20",
            CodeList::Recommendation21 => "CodeList::Recommendation21",
        }
    }
}
