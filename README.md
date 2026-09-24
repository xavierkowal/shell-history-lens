# shell-history-lens

A Rust library for turning shell history files into structured data. No
executable, no dependencies — just parsing.

## The problem

Every shell writes history to disk in its own dialect, and the dialects have
sharp edges:

- **bash** normally writes one command per line with nothing else. If
  `HISTTIMESTAMP` is set, each command is preceded by a comment line holding
  its Unix epoch (`#1690000000`) — but a `#`-prefixed line that isn't all
  digits is a real comment the user typed, not a timestamp.
- **zsh** with `EXTENDED_HISTORY` writes
  `: <start-time>:<elapsed-seconds>;<command>`. The command can itself
  contain colons and semicolons, so a naive split on `;` breaks on anything
  like `echo a; echo b`. Multi-line commands are encoded with a trailing
  backslash on every line but the last.
- **fish** writes a line-oriented format that looks like YAML but isn't
  parsed as such (`- cmd: ...`, `  when: ...`, `  paths:` block). The
  command text has its own escaping — a literal backslash is written as
  `\\` and a literal newline as `\n` — which has to be undone rather than
  passed through.

Grepping these files by hand works until one of those edge cases shows up.
This library parses them into a single `HistoryEntry { command, timestamp }`
shape so callers don't have to think about the source format.

## Usage

Add it as a path dependency (this crate isn't published):

```toml
[dependencies]
shell-history-lens = { path = "../shell-history-lens" }
```

```rust
use shell_history_lens::{parse_bash, parse_fish, parse_zsh_extended};

fn main() {
    let bash_history = "#1690000000\ngit status\nls -la\n";
    for entry in parse_bash(bash_history) {
        match entry.timestamp {
            Some(ts) => println!("[{ts}] {}", entry.command),
            None => println!("{}", entry.command),
        }
    }

    let zsh_history = ": 1690000000:0;echo a; echo b\n";
    for entry in parse_zsh_extended(zsh_history) {
        println!("{:?}", entry);
    }

    let fish_history = "- cmd: ls -la\n  when: 1690000000\n";
    for entry in parse_fish(fish_history) {
        println!("{:?}", entry);
    }

    // Not sure which shell wrote the file? `parse_auto` looks at the first
    // non-blank line and picks the matching parser.
    for entry in shell_history_lens::parse_auto(fish_history) {
        println!("{:?}", entry);
    }

    // Each `parse_*` function collects into a `Vec` up front. `iter_*`
    // returns the same entries lazily instead — `parse_bash` is just
    // `iter_bash(input).collect()` — so a caller can stop as soon as it has
    // what it needs without holding the whole file's worth of entries.
    let first_git_command = shell_history_lens::iter_bash(bash_history)
        .find(|entry| entry.command.starts_with("git"));
    println!("{first_git_command:?}");

    // Each format also serializes back the other way with `write_*`, so you
    // can filter or edit a history and hand back valid history text.
    let entries = parse_bash(bash_history);
    let rewritten = shell_history_lens::write_bash(&entries);
    assert_eq!(parse_bash(&rewritten), entries);
}
```

## What's here now

- `parse_bash` — plain and `HISTTIMESTAMP` bash history.
- `parse_zsh_extended` — zsh `EXTENDED_HISTORY`, including multi-line
  commands and semicolons embedded in the command text.
- `parse_fish` — fish's `fish_history` format, including its own
  backslash/newline escaping and `paths:` blocks.
- `detect` / `parse_auto` — guess the format from the file's first
  non-blank line and parse it accordingly. Fish and zsh's extended format
  both have an unambiguous marker on every line; anything else, including
  plain zsh history, is treated as bash.
- `iter_bash` / `iter_zsh_extended` / `iter_fish` / `iter_auto` — the same
  parsing, exposed as a lazy iterator instead of a `Vec`, for callers who
  want to stop early or avoid holding a whole large history file's worth of
  parsed entries at once.
- `write_bash` / `write_zsh_extended` / `write_fish` — the inverse of the
  three parsers: turn a slice of `HistoryEntry` back into that format's
  text. `write_zsh_extended` re-encodes multi-line commands and literal
  trailing backslashes the same way zsh itself does; `write_fish` redoes
  the `\\`/`\n` escaping. Round-tripping a file through `parse_*` and then
  the matching `write_*` reproduces equivalent entries (`write_fish` drops
  the `paths:` block, since `HistoryEntry` has nowhere to keep it).

## License

MIT, see `LICENSE`.
