use indicatif::{ProgressBar, ProgressStyle};

/// Creates a standard progress bar for import-style operations.
pub fn import_progress_bar(
    total: usize,
    message: impl Into<String>,
) -> anyhow::Result<ProgressBar> {
    let pb = ProgressBar::new(total as u64);

    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )?
        .progress_chars("#>-"),
    );

    pb.set_message(message.into());

    Ok(pb)
}
