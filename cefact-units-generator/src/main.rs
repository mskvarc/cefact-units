//! Generates `cefact-units/src/generated.rs` from the UN/CEFACT Recommendation No. 20 workbook in `sources/`.
//!
//! `generate` writes the file; `check` fails when the committed file differs from what the sources produce. The
//! published crate carries the generated file, so it has no build-time dependency on the workbook or its reader.

mod annex;
mod catalogue;
mod generated_file;
mod generation_error;
mod render;
mod source_file;
mod status;
mod workbook_row;

use crate::catalogue::Catalogue;
use crate::generation_error::GenerationError;
use crate::render::render;
use crate::source_file::REC20_REVISION_17;
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
        Command::Generate => generated_file::write(root, &rendered),
        Command::Check => generated_file::check(root, &rendered),
    }
}

/// Reads the sources under `root` and renders the generated file.
fn render_from_sources(root: &Path) -> Result<String, GenerationError> {
    let source = SourceFile::read(root, REC20_REVISION_17)?;
    let catalogue = Catalogue::from_rows(source.rows()?)?;
    render(&catalogue, &source)
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
    use crate::generated_file;
    use crate::generation_error::GenerationError;
    use crate::render_from_sources;
    use crate::workspace_root;
    use std::fs;

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
    fn the_committed_generated_file_is_fresh() {
        let fresh = workspace_root().and_then(|root| {
            let rendered = render_from_sources(root)?;
            generated_file::check(root, &rendered)
        });
        assert!(fresh.is_ok(), "{fresh:?}");
    }

    #[test]
    fn check_fails_on_a_stale_generated_file() {
        let outcome = workspace_root().and_then(|root| {
            let path = generated_file::path(root);
            let committed = fs::read_to_string(&path).map_err(|source| GenerationError::Io {
                path: path.clone(),
                source,
            })?;
            let mut stale = committed.as_bytes().to_vec();
            if let Some(last) = stale.last_mut() {
                *last = if *last == b'\n' { b' ' } else { b'\n' };
            }
            let stale = String::from_utf8_lossy(&stale);
            generated_file::verify_fresh(&committed, &stale, path)
        });
        assert!(
            matches!(outcome, Err(GenerationError::Stale { .. })),
            "{outcome:?}"
        );
    }
}
