use crate::cli::tui;

pub(crate) fn run_interactive_shell(db_arg: Option<String>) -> anyhow::Result<()> {
    tui::run_tui(db_arg).map_err(Into::into)
}

pub(crate) fn split_args(line: &str) -> anyhow::Result<Vec<String>> {
    Ok(shell_words::split(line)?)
}
