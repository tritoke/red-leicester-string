use clap::{Args, Parser, ValueEnum};
use core::fmt;
use lru::LruCache;
use memmap::Mmap;
use rayon::prelude::*;
use std::{
    fs::File,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{Mutex, atomic::AtomicBool},
};
use strided::Stride;

mod searcher;
use searcher::Searcher;

use crate::searcher::MatchDirection;

// GAMBLE is thread local so that it can be enabled on a per-test basis during testing
// this is fine for normal use as the main thread is always the thread building the NFA.
thread_local! {
    static GAMBLE: AtomicBool = AtomicBool::new(false);
}

/// Find flags automatically in CTF challenges.
/// This looks for flags in the provided files using searches similar to strings+grep,
/// but works even if the flag is transformed, e.g. encoded or xor-encrypted.
#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    haystack: Haystack,

    /// the number of directories down to search
    #[clap(long, requires = "directory")]
    max_depth: Option<usize>,

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

    /// don't print the flag if it doesn't end in } this prevents the output of potentially many
    /// partial flags in some cases
    #[clap(long, default_value_t = false)]
    strict: bool,

    /// How to output the flag
    #[clap(short, long, default_value = "with-context")]
    output: OutputMode,

    /// Enable absolutely every codec! this makes building the matching automaton about 100x slower!
    /// and can make searching around 4x slower
    #[clap(long, default_value_t = false)]
    gamble: bool,

    /// The string to look for instead of } when operating in strict mode.
    #[clap(long = "closing", default_value = "}")]
    closing_character: String,
    // TODO: support a flag regex as well
}

/// Defines the haystack we are going to search through
#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
struct Haystack {
    /// the file in which to search for flags
    #[clap(short, long)]
    file: Option<PathBuf>,

    /// the directory to search for flags in
    #[clap(short, long = "dir")]
    directory: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum OutputMode {
    /// Output only the flag
    FlagOnly,

    /// Output the flag and the context of where it was found
    WithContext,

    /// Output a JSON blob per line with the flag and its context
    JsonLines,
}

fn main() -> ExitCode {
    let args = Cli::parse();

    if args.patterns.is_empty() {
        eprintln!("patterns cannot be empty, please provide at least one pattern to search from");
        return ExitCode::FAILURE;
    }

    if args.gamble {
        GAMBLE.with(|gamble| gamble.store(true, std::sync::atomic::Ordering::Relaxed));
    }

    let before = std::time::Instant::now();
    let searcher = Searcher::new(args.patterns, &args.closing_character)
        .expect("Failed to build aho-corasick matcher for patterns");
    let took = std::time::Instant::now().duration_since(before);
    if args.verbose {
        eprintln!("Built the automaton in {took:?}")
    }

    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to build threadpool");
    }

    // ensure we don't spam too much with all the various encodings
    let cache_size = if args.verbose {
        // we don't cache seen flags in verbose mode
        unsafe { NonZeroUsize::new_unchecked(1) }
    } else {
        // memory is cheap, right???
        // also if you actually fill this message me lmao
        unsafe { NonZeroUsize::new_unchecked(10_000) }
    };
    let seen_flags = Mutex::new(LruCache::new(cache_size));

    let ctx = SearchContext {
        fast: args.fast,
        verbose: args.verbose,
        strict: args.strict,
        seen_flags,
        searcher,
        output: args.output,
    };

    let before = std::time::Instant::now();

    let exit_code = if let Some(file) = args.haystack.file {
        ctx.search(file)
    } else {
        let dir = args
            .haystack
            .directory
            .expect("clap forgot how to require arguments?");

        let mut walker = walkdir::WalkDir::new(dbg!(dir)).follow_links(true);
        if let Some(depth) = args.max_depth {
            walker = walker.max_depth(depth);
        }

        let mut exit_code = ExitCode::SUCCESS;
        for dir_entry in walker {
            let dir_entry = match dir_entry {
                Ok(de) => de,
                Err(e) => {
                    eprintln!("{e}");
                    continue;
                }
            };

            let Ok(meta) = dir_entry.metadata() else {
                continue;
            };

            // skip non-files and empty files
            if !meta.is_file() || meta.len() == 0 {
                continue;
            }

            if ctx.search(dir_entry.path()) != ExitCode::SUCCESS {
                exit_code = ExitCode::FAILURE;
            }
        }

        exit_code
    };

    let took = std::time::Instant::now().duration_since(before);
    if args.verbose {
        eprintln!("Found all flags in {took:?}")
    }

    exit_code
}

struct SearchContext {
    fast: bool,
    verbose: bool,
    strict: bool,
    seen_flags: Mutex<LruCache<String, ()>>,
    searcher: Searcher,
    output: OutputMode,
}

impl SearchContext {
    fn search(&self, file_name: impl AsRef<Path>) -> ExitCode {
        let file_name = file_name.as_ref();

        let file = match File::open(file_name) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to open {}: {e}", file_name.display());
                return ExitCode::FAILURE;
            }
        };

        let mmap = match unsafe { Mmap::map(&file) } {
            Ok(mmap) => mmap,
            Err(e) => {
                eprintln!("Failed to mmap {}: {e}", file_name.display());
                return ExitCode::FAILURE;
            }
        };

        let haystack = Stride::new(&mmap[..]);

        let max_stride = if self.fast { 8 } else { 32 };
        let mut piles = Vec::with_capacity(triangle(max_stride));
        for stride in 1..=max_stride {
            piles.extend(haystack.substrides(stride).enumerate());
        }

        let flags = piles.into_par_iter().flat_map_iter(|(offset, pile)| {
            let stride = pile.stride();
            self.searcher
                .search(pile)
                .map(move |(flag, decoder_name, match_direction)| Finding {
                    flag,
                    context: FlagContext {
                        decoder_name,
                        match_direction,
                        offset,
                        stride,
                    },
                })
        });

        let before = std::time::Instant::now();
        flags.for_each(|finding| {
            if self.strict && !finding.flag.ends_with('}') {
                return;
            }

            if let Ok(mut seen) = self.seen_flags.lock() {
                // report the flag if we are in verbose mode or if the flag is unseen recently
                if self.verbose || seen.put(finding.flag.clone(), ()).is_none() {
                    self.output.report(finding, file_name);
                }
            } else {
                eprintln!("Failed to acquire lock for seen flags, reporting all flags...");
                self.output.report(finding, file_name);
            }
        });
        let took = std::time::Instant::now().duration_since(before);

        if self.verbose {
            eprintln!("[{}] Finished search in {took:?}", file_name.display());
        }

        ExitCode::SUCCESS
    }
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

#[derive()]
struct Finding {
    flag: String,
    context: FlagContext,
}

impl Finding {
    fn to_json(&self) -> serde_json::Value {
        let match_direction = if self.context.match_direction == MatchDirection::Forward {
            "forward"
        } else {
            "backward"
        };

        serde_json::json!({
            "flag": self.flag,
            "decoder_name": self.context.decoder_name,
            "match_direction": match_direction,
            "offset": self.context.offset,
            "stride": self.context.stride,
        })
    }
}

impl OutputMode {
    fn report(&self, finding: Finding, file: &Path) {
        match self {
            OutputMode::FlagOnly => {
                println!("{}", finding.flag);
            }
            OutputMode::WithContext => {
                println!("[{}] {}:", file.display(), finding.context);
                println!("{}", finding.flag);
            }
            OutputMode::JsonLines => {
                let mut json = finding.to_json();
                json["file"] = serde_json::Value::String(file.display().to_string());
                println!("{}", json);
            }
        }
    }
}
