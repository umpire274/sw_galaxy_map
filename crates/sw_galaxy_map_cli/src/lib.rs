pub mod cli;
pub mod ui;

use crate::cli::args::Cli;
use anyhow::Result;
use clap::CommandFactory;

/// Render CLI help text for the provided topic arguments.
///
/// Supported examples:
/// - [] or ["--help"]
/// - ["route", "--help"]
/// - ["waypoint", "--help"]
pub fn render_help_for(args: &[String]) -> Result<String> {
    let mut cmd = Cli::command();

    if args.is_empty() || args == ["--help"] {
        let mut out = Vec::new();
        cmd.write_long_help(&mut out)?;
        let mut s = String::from_utf8(out).unwrap_or_default();
        if !s.ends_with("\n") {
            s.push('\n');
        }
        return Ok(s);
    }

    let mut current = &mut cmd;
    for token in args {
        if token == "--help" || token == "-h" {
            break;
        }
        if let Some(sub) = current.find_subcommand_mut(token) {
            current = sub;
        } else {
            let mut out = Vec::new();
            cmd.write_long_help(&mut out)?;
            let mut s = String::from_utf8(out).unwrap_or_default();
            if !s.ends_with("\n") {
                s.push('\n');
            }
            return Ok(s);
        }
    }

    let mut out = Vec::new();
    current.write_long_help(&mut out)?;
    let mut s = String::from_utf8(out).unwrap_or_default();
    if !s.ends_with("\n") {
        s.push('\n');
    }
    Ok(s)
}
