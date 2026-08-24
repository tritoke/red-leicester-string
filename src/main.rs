use clap::Parser;
use core::fmt;
use memmap::Mmap;
use rayon::prelude::*;
use std::{fs::File, path::PathBuf, process::ExitCode};
use strided::Stride;

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
    #[clap(short, long)]
    threads: Option<usize>,

    /// print the context of where the match was found, enable this for an output which is more
    /// similar to stringcheese
    #[clap(long, default_value_t = false)]
    context: bool,
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

    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to build threadpool");
    }

    let haystack = Stride::new(&mmap[..]);

    let max_stride = if args.fast { 8 } else { 32 };
    let mut piles = Vec::with_capacity(triangle(max_stride));

    for stride in 1..=max_stride {
        piles.extend(haystack.substrides(stride).enumerate());
    }

    let flags = piles.into_par_iter().flat_map_iter(|(offset, pile)| {
        let stride = pile.stride();
        searcher
            .search(pile)
            .map(move |(flag, decoder_name, match_direction)| {
                (
                    flag,
                    FlagContext {
                        decoder_name,
                        match_direction,
                        offset,
                        stride,
                    },
                )
            })
    });

    flags.for_each(|(flag, context)| {
        if args.context {
            println!("{context}:");
        }
        println!("{flag}");
    });

    ExitCode::SUCCESS
}

fn triangle(n: usize) -> usize {
    (n * (n + 1)) / 2
}

#[derive(Debug)]
struct FlagContext {
    decoder_name: &'static str,
    match_direction: MatchDirection,
    offset: usize,
    stride: usize,
}

impl fmt::Display for FlagContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Match found in stream")?;

        if self.stride != 1 {
            write!(f, "[{}::{}]", self.offset, self.stride)?;
        }

        if self.match_direction == MatchDirection::Backward {
            f.write_str("[::-1]")?
        }

        write!(f, " with decoder {}", self.decoder_name)
    }
}
