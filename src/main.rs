use aho_corasick::{AhoCorasick, Span};
use clap::Parser;
use std::{ffi::OsString, path::PathBuf};

mod searcher;
use searcher::Searcher;

/// Find flags automatically in CTF challenges.
/// This looks for flags in the provided files using searches similar to strings+grep,
/// but works even if the flag is transformed, e.g. encoded or xor-encrypted.
#[derive(Parser, Debug)]
struct Args {
    /// the file in which to search for flags, stdin by default
    #[clap(short, long)]
    file: PathBuf,

    /// skip the slow checks. Useful on larger files but you may miss matches
    #[clap(long)]
    fast: bool,

    /// increase output verbosity
    #[clap(short, long)]
    verbose: bool,

    /// the pattern you want to search, e.g. FLAG{
    patterns: Vec<OsString>,
}

fn main() {
    let args = Args::parse();
    dbg!(args);
}
