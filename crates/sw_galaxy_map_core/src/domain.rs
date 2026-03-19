use clap::ValueEnum;

/// Sorting strategy for persisted route listings.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum RouteListSort {
    Updated,
    Id,
    Length,
}
