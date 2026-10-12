//! Every way generation or the freshness check can fail.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why generation or the freshness check failed. Every variant names where the problem is.
#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    /// The command line named no known subcommand.
    #[error("usage: cefact-units-generator <generate|check>")]
    Usage,
    /// The generator's manifest directory has no parent, so the workspace root is unknown.
    #[error("the generator manifest directory has no parent directory")]
    NoWorkspaceRoot,
    /// A file could not be read or written.
    #[error("cannot access {path}")]
    Io {
        /// File that failed.
        path: PathBuf,
        /// Underlying failure.
        #[source]
        source: io::Error,
    },
    /// A source workbook or one of its sheets could not be opened.
    #[error("cannot read {path}")]
    Workbook {
        /// Source workbook.
        path: PathBuf,
        /// Underlying failure.
        #[source]
        source: calamine::XlsxError,
    },
    /// A sheet has no header row.
    #[error("sheet {sheet:?} is empty")]
    EmptySheet {
        /// Sheet name.
        sheet: &'static str,
    },
    /// A header cell does not name the column the generator reads from that position.
    #[error("sheet {sheet:?} column {column}: expected header {expected:?}, found {found:?}")]
    UnexpectedHeader {
        /// Sheet name.
        sheet: &'static str,
        /// Zero-based column index.
        column: usize,
        /// Header the layout expects.
        expected: &'static str,
        /// Header the sheet holds, whitespace collapsed.
        found: String,
    },
    /// A row carries a status marker the recommendation does not define.
    #[error("sheet {sheet:?} row {row}: unknown status marker {marker:?}")]
    UnknownStatusMarker {
        /// Sheet name.
        sheet: &'static str,
        /// One-based row number as the workbook shows it.
        row: u32,
        /// Marker found.
        marker: String,
    },
    /// A row lacks a cell every row must carry.
    #[error("sheet {sheet:?} row {row}: no {column}")]
    MissingCell {
        /// Sheet name.
        sheet: &'static str,
        /// One-based row number as the workbook shows it.
        row: u32,
        /// Column name.
        column: &'static str,
    },
    /// A cell holds a value of a kind no column of the recommendation carries.
    #[error("sheet {sheet:?} row {row} column {column}: unexpected cell {found}")]
    UnexpectedCell {
        /// Sheet name.
        sheet: &'static str,
        /// One-based row number as the workbook shows it.
        row: u32,
        /// Zero-based column index.
        column: usize,
        /// Debug rendering of the cell.
        found: String,
    },
    /// A common code is not 2 or 3 characters of `[A-Z0-9]`.
    #[error("sheet {sheet:?} row {row}: malformed common code {code:?}")]
    MalformedCode {
        /// Sheet name.
        sheet: &'static str,
        /// One-based row number as the workbook shows it.
        row: u32,
        /// Code found.
        code: String,
    },
    /// Two codes map to the same `UnitCode` variant name.
    #[error("codes {first:?} and {second:?} both name the variant {variant}")]
    VariantCollision {
        /// Variant name.
        variant: String,
        /// Code that took the name first.
        first: String,
        /// Code that collides with it.
        second: String,
    },
    /// Writing the generated source text failed.
    #[error("cannot render the generated file")]
    Render(#[from] fmt::Error),
    /// The committed generated file differs from what the sources produce.
    #[error("{path} is stale: run `cargo run -p cefact-units-generator -- generate`")]
    Stale {
        /// Generated file.
        path: PathBuf,
    },
}
