use crate::entry::HistoryEntry;

/// Parses a plain or `HISTTIMESTAMP`-enabled bash history file.
///
/// With `HISTTIMESTAMP` set, bash writes each command as a pair of lines:
/// a comment holding the epoch seconds (`#1690000000`), then the command
/// itself on the next line. Without it, every line is just a command.
/// A stray `#` line that isn't all digits is not a timestamp marker; bash
/// stores it verbatim because it's a real comment the user typed at the
/// prompt, so it comes back as a command with no timestamp attached.
pub fn parse(input: &str) -> Vec<HistoryEntry> {
    iter(input).collect()
}

/// Like [`parse`], but yields entries one at a time instead of collecting
/// them into a `Vec` up front. Lets a caller stop early or process a history
/// file too large to want fully materialized in memory.
pub fn iter(input: &str) -> BashHistory<'_> {
    BashHistory {
        lines: input.lines(),
        pending_timestamp: None,
    }
}

pub struct BashHistory<'a> {
    lines: std::str::Lines<'a>,
    pending_timestamp: Option<i64>,
}

impl<'a> Iterator for BashHistory<'a> {
    type Item = HistoryEntry;

    fn next(&mut self) -> Option<HistoryEntry> {
        for raw_line in self.lines.by_ref() {
            let line = raw_line.trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }

            if let Some(timestamp) = parse_timestamp_comment(line) {
                self.pending_timestamp = Some(timestamp);
                continue;
            }

            return Some(HistoryEntry {
                command: line.to_string(),
                timestamp: self.pending_timestamp.take(),
            });
        }

        None
    }
}

fn parse_timestamp_comment(line: &str) -> Option<i64> {
    let digits = line.strip_prefix('#')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{iter, parse};
    use crate::entry::HistoryEntry;

    struct Case {
        name: &'static str,
        input: &'static str,
        expected: Vec<HistoryEntry>,
    }

    fn entry(command: &str, timestamp: Option<i64>) -> HistoryEntry {
        HistoryEntry {
            command: command.to_string(),
            timestamp,
        }
    }

    #[test]
    fn table_driven_cases() {
        let cases = vec![
            Case {
                name: "plain command, no timestamp",
                input: "ls -la\n",
                expected: vec![entry("ls -la", None)],
            },
            Case {
                name: "timestamp comment applies to the next command only",
                input: "#1690000000\nls -la\npwd\n",
                expected: vec![entry("ls -la", Some(1690000000)), entry("pwd", None)],
            },
            Case {
                name: "non-numeric comment is a real command, not a timestamp",
                input: "# a note to self\n",
                expected: vec![entry("# a note to self", None)],
            },
            Case {
                name: "blank lines are skipped",
                input: "ls\n\n\npwd\n",
                expected: vec![entry("ls", None), entry("pwd", None)],
            },
            Case {
                name: "whitespace-only line is skipped",
                input: "ls\n   \npwd\n",
                expected: vec![entry("ls", None), entry("pwd", None)],
            },
            Case {
                name: "dangling timestamp at end of file is dropped",
                input: "ls\n#1690000000\n",
                expected: vec![entry("ls", None)],
            },
            Case {
                name: "crlf line endings are stripped",
                input: "ls -la\r\npwd\r\n",
                expected: vec![entry("ls -la", None), entry("pwd", None)],
            },
            Case {
                name: "empty input produces no entries",
                input: "",
                expected: vec![],
            },
        ];

        for case in cases {
            let got = parse(case.input);
            assert_eq!(got, case.expected, "case failed: {}", case.name);

            let got_from_iter: Vec<HistoryEntry> = iter(case.input).collect();
            assert_eq!(got_from_iter, case.expected, "iter case failed: {}", case.name);
        }
    }

    #[test]
    fn iter_stops_after_the_requested_number_of_entries() {
        let input = "#1690000000\nls -la\npwd\nwhoami\n";
        let first_two: Vec<HistoryEntry> = iter(input).take(2).collect();
        assert_eq!(
            first_two,
            vec![entry("ls -la", Some(1690000000)), entry("pwd", None)]
        );
    }
}
