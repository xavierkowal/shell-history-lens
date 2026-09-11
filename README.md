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
use shell_history_lens::{parse_bash, parse_zsh_extended};

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
}
```

## What's here now

- `parse_bash` — plain and `HISTTIMESTAMP` bash history.
- `parse_zsh_extended` — zsh `EXTENDED_HISTORY`, including multi-line
  commands and semicolons embedded in the command text.

Fish history and format auto-detection aren't implemented yet.

## License

MIT, see `LICENSE`.
