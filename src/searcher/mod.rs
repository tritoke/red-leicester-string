use std::sync::Arc;

use aho_corasick::{Match, automaton::Automaton, nfa};

mod codecs;
use codecs::{ALL_CODECS, Decoder, DecoderName};
use strided::Stride;

use crate::searcher::strided_ahocorasick::StridedFindIter;

mod strided_ahocorasick;

const MAX_FLAG_LENGTH: usize = 2000;
const CLOSING_CHAR: u8 = b'}';

// TODO: decide if I want this to be Clone for multithreading... #[derive(Clone)]
pub struct Searcher {
    matcher: Arc<dyn Automaton>,
    decoders: Vec<(Decoder, DecoderName, MatchDirection)>,
}

impl Searcher {
    pub fn new<I: IntoIterator<Item = P>, P: AsRef<[u8]>>(
        patterns: I,
    ) -> Result<Self, aho_corasick::BuildError> {
        let (patterns, decoders) = Self::expand_patterns(patterns);

        // Build a non-contiguous NFA (NNFA) and then try to upgrade it to contiguous one
        // if this fails just fall back to the NNFA
        let nnfa = nfa::noncontiguous::NFA::builder()
            // this doesn't work when striding
            .prefilter(false)
            .build(patterns)?;

        let builder = nfa::contiguous::Builder::new();
        let matcher: Arc<dyn Automaton> = match builder.build_from_noncontiguous(&nnfa) {
            Ok(cnfa) => Arc::new(cnfa),
            Err(_) => Arc::new(nnfa),
        };

        Ok(Self { matcher, decoders })
    }

    fn expand_patterns<I: IntoIterator<Item = P>, P: AsRef<[u8]>>(
        patterns: I,
    ) -> (Vec<Box<[u8]>>, Vec<(Decoder, &'static str, MatchDirection)>) {
        let mut encoded_patterns = vec![];
        let mut decoders = vec![];

        for pattern in patterns.into_iter() {
            let pat = pattern.as_ref();

            for codec in ALL_CODECS {
                let (encoded, name, decoder) = codec(pat);

                // also generate the backwards version of the pattern
                let mut reverse_encoded = Vec::from(encoded.clone());
                reverse_encoded.reverse();

                encoded_patterns.push(encoded);
                decoders.push((decoder, name, MatchDirection::Forward));

                encoded_patterns.push(Box::from(reverse_encoded));
                decoders.push((decoder, name, MatchDirection::Backward));
            }
        }

        assert_eq!(encoded_patterns.len(), decoders.len());
        (encoded_patterns, decoders)
    }

    fn find_iter<'a>(&self, haystack: Stride<'a, u8>) -> StridedFindIter<'a> {
        StridedFindIter::new(self.matcher.clone(), haystack)
            .expect("I fucked up creating the aho-corasick automaton, sorry...")
    }

    pub fn search<'a>(
        &self,
        haystack: Stride<'a, u8>,
    ) -> impl Iterator<Item = (String, DecoderName, MatchDirection)> {
        self.find_iter(haystack)
            .filter_map(move |match_| {
                // Look up the decoder for this match and whether it was a match going forwards or backwards
                let (decoder, name, direction) = self.decoders[match_.pattern().as_usize()];

                // Extend the match in the correct direcion, i.e.
                // if we matched "flag{" going forwards then walk forwards to grab more of the input
                // to find the rest of the flag
                let to_decode = Self::expand_search(haystack, match_, direction);
                let decoded = decoder(to_decode)?;
                Some((decoded, name, direction))
            })
            .map(|(decoded, name, direction)| {
                let flag = Self::postprocess_match(decoded);
                (flag, name, direction)
            })
    }

    fn expand_search<'a>(
        haystack: Stride<'a, u8>,
        match_: Match,
        direction: MatchDirection,
    ) -> Box<[u8]> {
        match direction {
            MatchDirection::Forward => {
                let start = match_.start();
                let end = usize::min(haystack.len(), match_.end() + MAX_FLAG_LENGTH);
                let slice = haystack.slice(start, end);
                let raw_match_data: Vec<u8> = slice.iter().copied().collect();
                raw_match_data.into()
            }
            MatchDirection::Backward => {
                let start = match_.start().saturating_sub(MAX_FLAG_LENGTH);
                let end = match_.end();
                let slice = haystack.slice(start, end);
                let raw_match_data: Vec<u8> = slice.iter().rev().copied().collect();
                raw_match_data.into()
            }
        }
    }

    fn postprocess_match(extended_match_data: Box<[u8]>) -> String {
        // compute where is valid UTF8 / where the closing brace is
        let haystack = extended_match_data.as_ref();
        let utf8_valid_end = encoding_rs::Encoding::utf8_valid_up_to(haystack);
        let closing_pos = memchr::memchr(CLOSING_CHAR, &haystack).unwrap_or(utf8_valid_end);

        // closing_pos is incremented as utf8_valid_end is an index valid for [..<to>] style
        // indexing but closing_pos is valid for [..=<to>] style indexing and this lets us use both
        // in the same place
        let truncate_to = usize::min(utf8_valid_end, closing_pos + 1);

        let mut owned = Vec::from(extended_match_data);
        // discard the invalid UTF8 data or until we see a closing brace
        owned.resize(truncate_to, 0);

        // not needed but we might as well I think
        owned.shrink_to_fit();

        // SAFETY: owned must contain only valid UTF8 data as either:
        // 1. we truncated to utf8_valid_end and thus all the data before this is utf8
        // 2. we hit closing_pos, which is strictly shorter than utf8_valid_end and is a
        //    unicode codepoint boundary and is thus safe to truncate to.
        unsafe { String::from_utf8_unchecked(owned) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchDirection {
    Forward,
    Backward,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, RngExt};
    use rstest::rstest;

    #[rstest]
    #[case("flag{", "flag{gimme_the_whole_flag}", 1, &["flag{gimme_the_whole_flag}"], MatchDirection::Forward)]
    #[case("flag{", "flag{short} with some garbage after", 1, &["flag{short}"], MatchDirection::Forward)]
    #[case("flag{", "and some trash before flag{short}", 1, &["flag{short}"], MatchDirection::Forward)]
    #[case("flag{", b"\xFF\xFF\xFFflag{short}\xFF\xFF\xFF", 1, &["flag{short}"], MatchDirection::Forward)]
    #[case("flag{", "}galf_elohw_eht_emmig{galf", 1, &["flag{gimme_the_whole_flag}"], MatchDirection::Backward)]
    #[case("flag{", "}trohs{galf with some garbage after", 1, &["flag{short}"], MatchDirection::Backward)]
    #[case("flag{", "and some trash before }trohs{galf", 1, &["flag{short}"], MatchDirection::Backward)]
    #[case("flag{", b"\xFF\xFF\xFF}trohs{galf\xFF\xFF\xFF", 1, &["flag{short}"], MatchDirection::Backward)]
    #[case("flag{", "ffllaagg{{ffllaagg12}}", 2, &["flag{flag1}", "flag{flag2}"], MatchDirection::Forward)]
    #[case("flag{", "ffffllaagg{{ffllaagg12}}", 2, &["flag{flag1}", "flag{flag2}"], MatchDirection::Forward)]
    #[case("flag{", "ffflllaaaggg{{{ffflllaaaggg123}}}", 3, &["flag{flag1}", "flag{flag2}", "flag{flag3}"], MatchDirection::Forward)]
    #[case("flag{", "}}}321gggaaalllfff{{{gggaaalllfff", 3, &["flag{flag3}", "flag{flag2}", "flag{flag1}"], MatchDirection::Backward)]
    #[case("flag{", "fAAAlAAAaAAAgAAA{AAAfAAAlAAAaAAAgAAA1AAA}AAA", 4, &["flag{flag1}"], MatchDirection::Forward)]
    #[case("flag{", "AAAAfAAAlAAAaAAAgAAA{AAAfAAAlAAAaAAAgAAA1AAA}AAA", 4, &["flag{flag1}"], MatchDirection::Forward)]
    fn test_identity_match(
        #[case] pattern: &str,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] stride_length: usize,
        #[case] correct: &[&str],
        #[case] correct_direction: MatchDirection,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let haystack = Stride::new(haystack.as_ref());
        let found: Vec<(String, DecoderName, MatchDirection)> = haystack
            .substrides(stride_length)
            .flat_map(|pile| searcher.search(pile))
            .collect();
        assert_eq!(found.len(), correct.len());

        for ((flag, decoder_name, match_direction), correct) in found.into_iter().zip(correct) {
            assert_eq!(&flag, correct);
            assert_eq!(decoder_name, "UTF8");
            assert_eq!(match_direction, correct_direction);
        }
    }

    // This test is kinda slow in debug mode so only run in release mode tests
    #[cfg_attr(debug_assertions, allow(dead_code))]
    #[cfg_attr(not(debug_assertions), test)]
    fn test_daft() {
        let mut buf = Vec::with_capacity(100_000);
        let mut rng = rand::rng();
        buf.resize(buf.capacity(), 0);

        let searcher = Searcher::new(["flag{"]).unwrap();

        let correct = "flag{big_stripe}";
        for stride in 2..(buf.len() / 2) / correct.len() {
            rng.fill_bytes(&mut buf[..]);

            let start = rng.random_range(0..buf.len() / 2);
            for (i, c) in correct.as_bytes().iter().enumerate() {
                buf[start + i * stride] = *c;
            }

            let haystack = Stride::new(&buf);
            let found: Vec<_> = haystack
                .substrides(stride)
                .skip(start % stride)
                .take(1)
                .flat_map(|pile| searcher.search(pile))
                .collect();

            assert_eq!(found.len(), 1);

            let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
            assert_eq!(flag, correct);
            assert_eq!(decoder_name, "UTF8");
            assert_eq!(match_direction, MatchDirection::Forward);
        }
    }
}
