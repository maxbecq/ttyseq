//! ttySeq — terminal-driven hybrid audio/MIDI/CV sequencer for live performance.
//!
//! Single binary with several launch modes: engine + embedded TUI by default,
//! `daemon` (headless), `attach` (remote TUI client) and one-shot subcommands
//! to come — see doc/spec/spec.md §3.3.

fn main() {
    println!("ttyseq {}", env!("CARGO_PKG_VERSION"));
}
