//! Generates `cefact-units/src/generated.rs` from the UN/CEFACT Recommendation No. 20 and No. 21 workbooks in
//! `sources/`, and appends every newly listed code to the identity registry `cefact-units/data/ids.tsv`.
//!
//! `generate` writes both files; `check` fails when a committed file differs from what the sources produce. The
//! published crate carries the generated files, so it has no build-time dependency on the workbook or its reader.

mod annex;
mod catalogue;
mod code_list;
mod generated_file;
mod generation_error;
mod identity_registry;
mod lifecycle;
mod marker_erratum;
mod packaging_row;
mod render;
mod sheet_text;
mod source_file;
mod status;
mod workbook_row;

use crate::catalogue::Catalogue;
use crate::code_list::CodeList;
use crate::generated_file::IDENTITY_REGISTRY;
use crate::generated_file::RenderedFiles;
use crate::generation_error::GenerationError;
use crate::identity_registry::IdentityRegistry;
use crate::lifecycle::RevisionRows;
use crate::render::render;
use crate::source_file::SOURCES;
use crate::source_file::SourceFile;
use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

/// What the generator was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    /// Write the generated file.
    Generate,
    /// Fail when the committed generated file is stale.
    Check,
}

impl Command {
    /// Reads the subcommand from the arguments after the program name.
    fn from_args(mut args: impl Iterator<Item = String>) -> Result<Command, GenerationError> {
        match (args.next().as_deref(), args.next()) {
            (Some("generate"), None) => Ok(Command::Generate),
            (Some("check"), None) => Ok(Command::Check),
            (Some(_) | None, _) => Err(GenerationError::Usage),
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            let mut cause = error.source();
            while let Some(source) = cause {
                eprintln!("  caused by: {source}");
                cause = source.source();
            }
            ExitCode::FAILURE
        }
    }
}

/// Runs the subcommand the arguments name.
fn run() -> Result<(), GenerationError> {
    let command = Command::from_args(std::env::args().skip(1))?;
    let root = workspace_root()?;
    let rendered = render_from_sources(root)?;
    match command {
        Command::Generate => rendered.write(root),
        Command::Check => rendered.check(root),
    }
}

/// Reads every source and the identity registry under `root` and renders every generated file.
fn render_from_sources(root: &Path) -> Result<RenderedFiles, GenerationError> {
    let sources: Vec<SourceFile> = SOURCES
        .iter()
        .map(|spec| SourceFile::read(root, *spec))
        .collect::<Result<_, _>>()?;
    let mut revisions = Vec::new();
    let mut packaging = Vec::new();
    for source in &sources {
        match source.code_list {
            CodeList::Recommendation20 => revisions.push(RevisionRows {
                revision: source.revision,
                rows: source.rows()?,
            }),
            CodeList::Recommendation21 => packaging.extend(source.packaging_rows()?),
        }
    }
    let catalogue = Catalogue::build(revisions, packaging)?;
    let registry = IdentityRegistry::parse(&IDENTITY_REGISTRY.read(root)?)?;
    let assignment = registry.assign(&catalogue.units)?;
    Ok(RenderedFiles {
        unit_tables: render(&assignment.units, &sources)?,
        identity_registry: assignment.registry.render(),
    })
}

/// The workspace root: the parent of this crate's manifest directory.
fn workspace_root() -> Result<&'static Path, GenerationError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or(GenerationError::NoWorkspaceRoot)
}

#[cfg(test)]
mod tests {
    use crate::Command;
    use crate::generated_file::UNIT_TABLES;
    use crate::generated_file::verify_fresh;
    use crate::generation_error::GenerationError;
    use crate::render_from_sources;
    use crate::workspace_root;

    fn args(words: &[&str]) -> impl Iterator<Item = String> {
        words
            .iter()
            .map(|word| (*word).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn the_subcommand_is_read_from_the_first_argument() {
        assert_eq!(
            Command::from_args(args(&["generate"])).ok(),
            Some(Command::Generate)
        );
        assert_eq!(
            Command::from_args(args(&["check"])).ok(),
            Some(Command::Check)
        );
    }

    #[test]
    fn a_missing_unknown_or_extended_subcommand_is_a_usage_error() {
        for words in [&[][..], &["build"], &["check", "extra"]] {
            assert!(
                matches!(Command::from_args(args(words)), Err(GenerationError::Usage)),
                "{words:?}"
            );
        }
    }

    #[test]
    fn the_committed_generated_files_are_fresh() {
        let fresh = workspace_root().and_then(|root| render_from_sources(root)?.check(root));
        assert!(fresh.is_ok(), "{fresh:?}");
    }

    #[test]
    fn check_fails_on_a_stale_generated_file() {
        let outcome = workspace_root().and_then(|root| {
            let committed = UNIT_TABLES.read(root)?;
            let mut stale = committed.as_bytes().to_vec();
            if let Some(last) = stale.last_mut() {
                *last = if *last == b'\n' { b' ' } else { b'\n' };
            }
            let stale = String::from_utf8_lossy(&stale);
            verify_fresh(&committed, &stale, UNIT_TABLES.path(root))
        });
        assert!(
            matches!(outcome, Err(GenerationError::Stale { .. })),
            "{outcome:?}"
        );
    }
}
