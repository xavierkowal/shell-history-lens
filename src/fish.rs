use crate::entry::HistoryEntry;

/// Parses a fish `fish_history` file.
///
/// Fish stores history as a flat, line-oriented format that only looks like
/// YAML. Each entry starts with `- cmd: <escaped command>`, optionally
/// followed by a `  when: <epoch seconds>` line and a `  paths:` block
/// listing files fish associated with the command (used for its own
/// autocomplete bookkeeping, irrelevant here, so it's skipped). Within the
/// command text fish escapes a literal backslash as `\\` and a literal
/// newline as `\n`; those are undone here rather than left for the caller
/// to deal with. Lines that aren't part of a `- cmd:` entry are ignored.
pub fn parse(input: &str) -> Vec<HistoryEntry> {
    iter(input).collect()
}

/// Like [`parse`], but yields entries one at a time instead of collecting
/// them into a `Vec` up front. Lets a caller stop early or process a history
/// file too large to want fully materialized in memory.
pub fn iter(input: &str) -> FishHistory<'_> {
    FishHistory {
        lines: input.lines().peekable(),
    }
}

pub struct FishHistory<'a> {
    lines: std::iter::Peekable<std::str::Lines<'a>>,
}

impl<'a> Iterator for FishHistory<'a> {
    type Item = HistoryEntry;

    fn next(&mut self) -> Option<HistoryEntry> {
        while let Some(raw_line) = self.lines.next() {
            let line = raw_line.trim_end_matches('\r');
            let Some(escaped_command) = line.strip_prefix("- cmd: ") else {
                continue;
            };

            let command = unescape(escaped_command);
            let mut timestamp = None;

            while let Some(next_raw) = self.lines.peek() {
                let next_line = next_raw.trim_end_matches('\r');
                if let Some(when) = next_line.strip_prefix("  when: ") {
                    timestamp = when.trim().parse().ok();
                    self.lines.next();
                } else if next_line == "  paths:" {
                    self.lines.next();
                    while let Some(path_raw) = self.lines.peek() {
                        if path_raw.trim_end_matches('\r').starts_with("    - ") {
                            self.lines.next();
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }

            return Some(HistoryEntry { command, timestamp });
        }

        None
    }
}

fn unescape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }

        match chars.next() {
            Some('n') => result.push('\n'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    result
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
                name: "basic entry with timestamp",
                input: "- cmd: ls -la\n  when: 1690000000\n",
                expected: vec![entry("ls -la", Some(1690000000))],
            },
            Case {
                name: "entry with a paths block is still parsed correctly",
                input: "- cmd: cat foo.txt\n  when: 1690000000\n  paths:\n    - foo.txt\n",
                expected: vec![entry("cat foo.txt", Some(1690000000))],
            },
            Case {
                name: "escaped newline becomes a real newline",
                input: "- cmd: echo one\\nand two\n  when: 1690000000\n",
                expected: vec![entry("echo one\nand two", Some(1690000000))],
            },
            Case {
                name: "escaped backslash becomes a single backslash",
                input: "- cmd: echo a\\\\b\n  when: 1690000000\n",
                expected: vec![entry("echo a\\b", Some(1690000000))],
            },
            Case {
                name: "unrecognized escape sequence is kept literally",
                input: "- cmd: echo a\\qb\n  when: 1690000000\n",
                expected: vec![entry("echo a\\qb", Some(1690000000))],
            },
            Case {
                name: "cmd without a when line has no timestamp",
                input: "- cmd: ls -la\n",
                expected: vec![entry("ls -la", None)],
            },
            Case {
                name: "multiple entries in sequence",
                input: "- cmd: ls\n  when: 1690000000\n- cmd: pwd\n  when: 1690000001\n",
                expected: vec![
                    entry("ls", Some(1690000000)),
                    entry("pwd", Some(1690000001)),
                ],
            },
            Case {
                name: "lines outside an entry are ignored",
                input: "some stray header\n- cmd: ls\n  when: 1690000000\n",
                expected: vec![entry("ls", Some(1690000000))],
            },
            Case {
                name: "crlf line endings are stripped",
                input: "- cmd: ls -la\r\n  when: 1690000000\r\n",
                expected: vec![entry("ls -la", Some(1690000000))],
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
        let input = "- cmd: ls\n  when: 1690000000\n- cmd: pwd\n  when: 1690000001\n- cmd: whoami\n  when: 1690000002\n";
        let first_two: Vec<HistoryEntry> = iter(input).take(2).collect();
        assert_eq!(
            first_two,
            vec![
                entry("ls", Some(1690000000)),
                entry("pwd", Some(1690000001)),
            ]
        );
    }
}
