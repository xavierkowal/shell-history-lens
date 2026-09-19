use crate::entry::HistoryEntry;
use crate::{bash, fish, zsh};

/// Which shell wrote a history file, as far as [`detect`] can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryFormat {
    /// Plain or `HISTTIMESTAMP` bash history, and anything that doesn't
    /// match the other two dialects (including plain zsh history, which is
    /// byte-for-byte the same as plain bash history).
    Bash,
    ZshExtended,
    Fish,
}

/// Guesses which dialect `input` is written in by looking at the first
/// non-blank line.
///
/// Fish and zsh's `EXTENDED_HISTORY` both have an unambiguous marker on
/// every entry (`- cmd: ` and `: <start>:<elapsed>;` respectively), so one
/// line is enough to tell them apart. Plain bash has no marker at all, so
/// it's the fallback for anything that isn't clearly one of the other two.
pub fn detect(input: &str) -> HistoryFormat {
    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        if line.starts_with("- cmd: ") {
            return HistoryFormat::Fish;
        }
        if zsh::looks_like_extended_prefix(line) {
            return HistoryFormat::ZshExtended;
        }
        break;
    }

    HistoryFormat::Bash
}

/// Detects the format of `input` and parses it with the matching parser.
pub fn parse_auto(input: &str) -> Vec<HistoryEntry> {
    match detect(input) {
        HistoryFormat::Bash => bash::parse(input),
        HistoryFormat::ZshExtended => zsh::parse(input),
        HistoryFormat::Fish => fish::parse(input),
    }
}

/// Like [`parse_auto`], but yields entries one at a time instead of
/// collecting them into a `Vec` up front. The concrete iterator type differs
/// per format, so this returns a boxed trait object rather than exposing it.
pub fn iter_auto(input: &str) -> Box<dyn Iterator<Item = HistoryEntry> + '_> {
    match detect(input) {
        HistoryFormat::Bash => Box::new(bash::iter(input)),
        HistoryFormat::ZshExtended => Box::new(zsh::iter(input)),
        HistoryFormat::Fish => Box::new(fish::iter(input)),
    }
}

#[cfg(test)]
mod tests {
    use super::{detect, iter_auto, parse_auto, HistoryFormat};
    use crate::entry::HistoryEntry;

    struct Case {
        name: &'static str,
        input: &'static str,
        expected: HistoryFormat,
    }

    #[test]
    fn table_driven_detect_cases() {
        let cases = vec![
            Case {
                name: "plain bash history",
                input: "ls -la\npwd\n",
                expected: HistoryFormat::Bash,
            },
            Case {
                name: "bash history with HISTTIMESTAMP",
                input: "#1690000000\nls -la\n",
                expected: HistoryFormat::Bash,
            },
            Case {
                name: "zsh extended history",
                input: ": 1690000000:0;ls -la\n",
                expected: HistoryFormat::ZshExtended,
            },
            Case {
                name: "fish history",
                input: "- cmd: ls -la\n  when: 1690000000\n",
                expected: HistoryFormat::Fish,
            },
            Case {
                name: "leading blank lines are skipped before detecting",
                input: "\n\n- cmd: ls -la\n  when: 1690000000\n",
                expected: HistoryFormat::Fish,
            },
            Case {
                name: "empty input falls back to bash",
                input: "",
                expected: HistoryFormat::Bash,
            },
            Case {
                name: "all-blank input falls back to bash",
                input: "\n   \n",
                expected: HistoryFormat::Bash,
            },
        ];

        for case in cases {
            let got = detect(case.input);
            assert_eq!(got, case.expected, "case failed: {}", case.name);
        }
    }

    #[test]
    fn parse_auto_dispatches_to_the_detected_parser() {
        let fish_history = "- cmd: ls -la\n  when: 1690000000\n";
        assert_eq!(
            parse_auto(fish_history),
            vec![HistoryEntry {
                command: "ls -la".to_string(),
                timestamp: Some(1690000000),
            }]
        );

        let zsh_history = ": 1690000000:0;echo a; echo b\n";
        assert_eq!(
            parse_auto(zsh_history),
            vec![HistoryEntry {
                command: "echo a; echo b".to_string(),
                timestamp: Some(1690000000),
            }]
        );
    }

    #[test]
    fn iter_auto_dispatches_to_the_detected_parser() {
        let fish_history = "- cmd: ls -la\n  when: 1690000000\n";
        let got: Vec<HistoryEntry> = iter_auto(fish_history).collect();
        assert_eq!(got, parse_auto(fish_history));

        let zsh_history = ": 1690000000:0;echo a; echo b\n";
        let got: Vec<HistoryEntry> = iter_auto(zsh_history).collect();
        assert_eq!(got, parse_auto(zsh_history));

        let bash_history = "#1690000000\nls -la\n";
        let got: Vec<HistoryEntry> = iter_auto(bash_history).collect();
        assert_eq!(got, parse_auto(bash_history));
    }
}
