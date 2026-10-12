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
        source: calamine::Error,
    },
    /// A row of a source workbook failed to read.
    #[error("in {path}")]
    InSource {
        /// Source workbook.
        path: PathBuf,
        /// The row failure.
        #[source]
        source: Box<GenerationError>,
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
    /// A revision no longer lists a code an earlier revision listed. The recommendation withdraws a code by marking it
    /// deleted, never by dropping its row.
    #[error("revision {revision} no longer lists code {code:?}")]
    DroppedCode {
        /// Code dropped.
        code: String,
        /// First revision no longer listing it.
        revision: u16,
    },
    /// No Recommendation No. 20 source is configured.
    #[error("no Recommendation No. 20 source is configured")]
    NoRecommendation20Source,
    /// An identity registry line is not `<identity>\t<code>` with a numeric identity and a well-formed code.
    #[error("identity registry line {line}: expected `<identity>\\t<code>`, found {text:?}")]
    MalformedRegistryLine {
        /// One-based line number.
        line: usize,
        /// Line found.
        text: String,
    },
    /// An identity registry line carries an identity other than its line number.
    #[error("identity registry line {line}: identity {found} is not the line number")]
    IdentityOutOfSequence {
        /// One-based line number.
        line: usize,
        /// Identity found.
        found: u16,
    },
    /// The identity registry lists a code twice.
    #[error("identity registry lists code {code:?} twice")]
    DuplicateRegisteredCode {
        /// Code registered twice.
        code: String,
    },
    /// The identity registry holds a code no source lists. Codes are never removed from the registry.
    #[error("identity registry holds code {code:?}, which no source lists")]
    UnlistedRegisteredCode {
        /// Registered code.
        code: String,
    },
    /// Registering a code would need an identity past the signed 16-bit `SMALLINT` range.
    #[error("code {code:?} would take an identity past 32767")]
    IdentityOverflow {
        /// Code that does not fit.
        code: String,
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
