use clap::Parser;
use memmap::Mmap;
use std::{fs::File, io::Write, path::PathBuf, process::ExitCode};

mod codecs;

mod searcher;
use searcher::Searcher;

use crate::searcher::MatchDirection;

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
    patterns: Vec<String>,

    /// the number of threads to use while searching
    threads: Option<u32>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.patterns.is_empty() {
        eprintln!("patterns cannot be empty, please provide at least one pattern to search from");
        return ExitCode::FAILURE;
    }

    let file = File::open(args.file).expect("Failed to open file");
    let mmap = unsafe { Mmap::map(&file) }.expect("Failed to mmap file");

    let searcher =
        Searcher::new(args.patterns).expect("Failed to build aho-corasick matcher for patterns");

    let mut stdout = std::io::stdout().lock();
    for (flag, decoder_name, match_direction) in searcher.search(&mmap[..]) {
        let direction = if match_direction == MatchDirection::Forward {
            "->"
        } else {
            "<-"
        };
        writeln!(stdout, "[{decoder_name}::{direction}] {flag}")
            .expect("Failed to write to stdout??");
    }

    ExitCode::SUCCESS
}
