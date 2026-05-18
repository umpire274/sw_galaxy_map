use indicatif::{ProgressBar, ProgressStyle};

/// Creates a standard progress bar for import-style operations.
pub fn import_progress_bar(
    total: usize,
    message: impl Into<String>,
) -> anyhow::Result<ProgressBar> {
    let pb = ProgressBar::new(total as u64);

    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] \
                        {pos}/{len} ({percent:>3}%) ETA:{eta_precise} {msg}",
        )?
        .progress_chars("#>-"),
    );

    pb.set_message(message.into());

    Ok(pb)
}

pub fn spinner(message: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();

    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}").unwrap(),
    );

    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_message(message.into());

    pb
}
