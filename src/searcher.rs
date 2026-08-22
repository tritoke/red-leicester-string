use aho_corasick::{AhoCorasick, automaton::FindOverlappingIter};

struct SearchBuilder {
    matcher: AhoCorasick,
}

impl SearchBuilder {
    fn new<I: IntoIterator<Item = P>, P: AsRef<[u8]>>(
        patterns: I,
    ) -> Result<Self, aho_corasick::BuildError> {
        matcher = AhoCorasick::new(patterns)?;
        Ok(Self { matcher })
    }

    fn search<'a, 'h, I: Into<Input<'h>>>(&self, input: I) {}
}

fn search(
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
