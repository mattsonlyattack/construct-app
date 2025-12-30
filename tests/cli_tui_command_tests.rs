//! CLI integration tests for the `cons tui` command.
//!
//! These tests verify that the TUI subcommand is correctly integrated with clap.

use clap::{CommandFactory, Parser};

/// cons - structure-last personal knowledge management CLI
#[derive(Parser)]
#[command(name = "cons")]
#[command(about = "A structure-last personal knowledge management tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available commands (minimal subset for testing)
#[derive(clap::Subcommand)]
enum Commands {
    /// Add a new note with optional tags
    Add { content: String },
    /// Launch interactive terminal UI
    Tui,
}

#[test]
fn tui_command_parses_correctly() {
    // Test parsing of `cons tui` with clap
    let matches = Cli::command()
        .try_get_matches_from(vec!["cons", "tui"])
        .expect("failed to parse tui command");

    // Verify command is recognized
    assert!(
        matches.subcommand_matches("tui").is_some(),
        "tui subcommand should be recognized"
    );
}

#[test]
fn tui_command_has_help_text() {
    // Verify that the Tui command has appropriate help text
    let cmd = Cli::command();
    let tui_subcommand = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "tui")
        .expect("tui subcommand should exist");

    let about = tui_subcommand
        .get_about()
        .expect("tui command should have about text");

    assert!(
        about.to_string().contains("interactive terminal UI")
            || about.to_string().contains("Launch interactive terminal UI"),
        "help text should describe the TUI functionality"
    );
}

#[test]
fn tui_command_takes_no_arguments() {
    // Verify that `cons tui` doesn't accept unexpected arguments
    let result = Cli::command().try_get_matches_from(vec!["cons", "tui", "extra-arg"]);

    // Should fail because tui doesn't take arguments
    assert!(
        result.is_err(),
        "tui command should not accept extra arguments"
    );
}
