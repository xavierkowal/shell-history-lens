//! Parsers for shell history files.
//!
//! Every shell writes its history to disk in its own dialect. This crate
//! turns those dialects into a single [`HistoryEntry`] shape so callers
//! don't have to special-case bash vs zsh themselves.

mod bash;
mod entry;
mod zsh;

pub use bash::parse as parse_bash;
pub use entry::HistoryEntry;
pub use zsh::parse as parse_zsh_extended;
