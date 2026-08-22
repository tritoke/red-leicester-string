use aho_corasick::{AhoCorasick, Span};
use clap::Parser;
use std::{ffi::OsString, path::PathBuf};

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
    pattern: OsString,
}

fn main() {
    let args = Args::parse();
    dbg!(args);
}

fn extend_match_till_null(haystack: &[u8], span: Span) -> &[u8] {
    match haystack[span.start..].iter().position(|b| *b == 0) {
        Some(null) => &haystack[span.start..null],
        None => &haystack[span.start..],
    }
}

fn search(
    patterns: &'static [&'static [u8]],
    haystack: &'static [u8],
) -> Result<impl Iterator<Item = &'static [u8]>, aho_corasick::BuildError> {
    let matcher = AhoCorasick::new(patterns)?;

    let matches = matcher
        .find_overlapping_iter(haystack)
        .map(|m| extend_match_till_null(haystack, m.span()));

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("flag{", "flag{gimme_the_whole_flag}", "flag{gimme_the_whole_flag}")]
    fn test_identity_match<P: AsRef<[u8]>>(
        #[case] pattern: P,
        #[case] haystack: P,
        #[case] correct: P,
    ) {
        // search();
    }
}
