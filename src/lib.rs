//! Parsers for shell history files.
//!
//! Every shell writes its history to disk in its own dialect. This crate
//! turns those dialects into a single [`HistoryEntry`] shape so callers
//! don't have to special-case bash vs zsh themselves.

mod bash;
mod entry;
mod fish;
mod format;
mod zsh;

pub use bash::{iter as iter_bash, parse as parse_bash, write as write_bash, BashHistory};
pub use entry::HistoryEntry;
pub use fish::{iter as iter_fish, parse as parse_fish, write as write_fish, FishHistory};
pub use format::{detect, iter_auto, parse_auto, HistoryFormat};
pub use zsh::{
    iter as iter_zsh_extended, parse as parse_zsh_extended, write as write_zsh_extended,
    ZshHistory,
};
