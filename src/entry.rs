/// A single command pulled out of a shell history file.
///
/// `timestamp` is a Unix epoch (seconds) when the source format records one.
/// Plain bash history without `HISTTIMESTAMP`, and any line that doesn't
/// carry a parseable timestamp of its own, comes back as `None` rather than
/// a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: Option<i64>,
}
