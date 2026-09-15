use crate::entry::HistoryEntry;

/// Parses a zsh `EXTENDED_HISTORY` file.
///
/// Each entry looks like `: <start-time>:<elapsed-seconds>;<command>`. The
/// command itself is free text and may contain colons or semicolons, so
/// only the first semicolon after the two numeric fields is treated as the
/// separator. A command that spans multiple lines has every line but the
/// last end in a trailing backslash; those continuation lines carry no
/// prefix of their own and get folded back into the command with the
/// original newlines restored. A line that doesn't match the extended
/// format at all (SHARE_HISTORY was off when it was written, say) is kept
/// as a plain command with no timestamp.
///
/// A command that itself ends in a literal backslash is written by zsh with
/// that backslash doubled, so a run of trailing backslashes is only a
/// continuation marker when its length is odd: one backslash means "join the
/// next line", two mean "the command really ends in `\`", three mean a
/// literal `\` followed by a continuation, and so on.
pub fn parse(input: &str) -> Vec<HistoryEntry> {
    let mut entries = Vec::new();
    let mut lines = input.lines();

    while let Some(raw_line) = lines.next() {
        let line = raw_line.trim_end_matches('\r');

        if let Some((timestamp, mut command)) = parse_extended_prefix(line) {
            loop {
                // Collapse the trailing run of backslashes down to half its
                // length (each escaped pair is one literal backslash). An
                // odd run has one marker backslash left over after pairing
                // off the rest, which means "join the next line".
                let run = trailing_backslash_count(&command);
                command.truncate(command.len() - run);
                for _ in 0..run / 2 {
                    command.push('\\');
                }
                if run % 2 == 0 {
                    break;
                }
                match lines.next() {
                    Some(next_raw) => {
                        command.push('\n');
                        command.push_str(next_raw.trim_end_matches('\r'));
                    }
                    None => break,
                }
            }
            entries.push(HistoryEntry {
                command,
                timestamp: Some(timestamp),
            });
        } else {
            if line.trim().is_empty() {
                continue;
            }
            entries.push(HistoryEntry {
                command: line.to_string(),
                timestamp: None,
            });
        }
    }

    entries
}

/// Whether `line` starts with a well-formed `: <start>:<elapsed>;` prefix,
/// without caring about the command text that follows. Used by format
/// detection, which only needs to know if a line looks like this dialect.
pub(crate) fn looks_like_extended_prefix(line: &str) -> bool {
    parse_extended_prefix(line).is_some()
}

fn trailing_backslash_count(command: &str) -> usize {
    command.chars().rev().take_while(|&c| c == '\\').count()
}

fn parse_extended_prefix(line: &str) -> Option<(i64, String)> {
    let rest = line.strip_prefix(": ")?;

    let colon = rest.find(':')?;
    let (start_str, rest) = rest.split_at(colon);
    let timestamp: i64 = start_str.parse().ok()?;
    let rest = &rest[1..];

    let semi = rest.find(';')?;
    let (elapsed_str, rest) = rest.split_at(semi);
    if elapsed_str.is_empty() || !elapsed_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    Some((timestamp, rest[1..].to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse;
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
                name: "basic extended entry",
                input: ": 1690000000:0;ls -la\n",
                expected: vec![entry("ls -la", Some(1690000000))],
            },
            Case {
                name: "semicolon inside the command is kept intact",
                input: ": 1690000000:0;echo a; echo b\n",
                expected: vec![entry("echo a; echo b", Some(1690000000))],
            },
            Case {
                name: "plain line without extended prefix has no timestamp",
                input: "ls -la\n",
                expected: vec![entry("ls -la", None)],
            },
            Case {
                name: "multi-line command joins continuation lines",
                input: ": 1690000000:0;echo one\\\nand two\n",
                expected: vec![entry("echo one\nand two", Some(1690000000))],
            },
            Case {
                name: "doubled trailing backslash is a literal, not a continuation",
                input: ": 1690000000:0;echo one\\\\\n",
                expected: vec![entry("echo one\\", Some(1690000000))],
            },
            Case {
                name: "literal backslash followed by a real continuation",
                input: ": 1690000000:0;echo one\\\\\\\nand two\n",
                expected: vec![entry("echo one\\\nand two", Some(1690000000))],
            },
            Case {
                name: "continuation marker missing its next line is dropped",
                input: ": 1690000000:0;echo one\\",
                expected: vec![entry("echo one", Some(1690000000))],
            },
            Case {
                name: "empty command after the separator",
                input: ": 1690000000:0;\n",
                expected: vec![entry("", Some(1690000000))],
            },
            Case {
                name: "non-numeric timestamp falls back to a plain command",
                input: ": abc:0;ls\n",
                expected: vec![entry(": abc:0;ls", None)],
            },
            Case {
                name: "non-numeric elapsed field falls back to a plain command",
                input: ": 1690000000:xy;ls\n",
                expected: vec![entry(": 1690000000:xy;ls", None)],
            },
            Case {
                name: "crlf line endings are stripped",
                input: ": 1690000000:0;ls -la\r\n",
                expected: vec![entry("ls -la", Some(1690000000))],
            },
            Case {
                name: "blank plain line is skipped",
                input: "\n",
                expected: vec![],
            },
        ];

        for case in cases {
            let got = parse(case.input);
            assert_eq!(got, case.expected, "case failed: {}", case.name);
        }
    }
}
