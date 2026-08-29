use std::{
    fmt::Debug,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::Arc,
};

use aho_corasick::{Match, automaton::Automaton, nfa};

mod codecs;
use codecs::{ALL_CODEC_GENERATORS, DecoderName};
use strided::Stride;

use crate::searcher::{
    codecs::{Codec, DecoderMetadata, Encoded, MaybeDecoded},
    strided_ahocorasick::StridedFindOverlapping,
};

mod strided_ahocorasick;

const MAX_FLAG_LENGTH: usize = 2000;
const CLOSING_CHAR: u8 = b'}';

// Also nicked from burntsushi (if you ever read this thanks for writing an awesome library
// this has been a joy to work on!)
trait AcAutomaton: Automaton + Debug + Send + Sync + UnwindSafe + RefUnwindSafe + 'static {}

impl<A> AcAutomaton for A where
    A: Automaton + Debug + Send + Sync + UnwindSafe + RefUnwindSafe + 'static
{
}

struct DecodingContext {
    decoder: fn(Encoded, DecoderMetadata) -> MaybeDecoded,
    name: &'static str,
    metadata: DecoderMetadata,
    match_direction: MatchDirection,
}

pub struct Searcher {
    matcher: Arc<dyn AcAutomaton>,
    decoder_mapping: Vec<DecodingContext>,
    unexpanded: Box<[String]>,
}

impl Searcher {
    pub fn new(
        patterns: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, aho_corasick::BuildError> {
        let unexpanded: Vec<_> = patterns
            .into_iter()
            .map(|pat| pat.as_ref().to_owned())
            .collect();
        let (patterns, decoder_mapping) = Self::expand_patterns(unexpanded.clone());

        // Build a non-contiguous NFA (NNFA) and then try to upgrade it to contiguous one
        // if this fails just fall back to the NNFA
        let nnfa = nfa::noncontiguous::NFA::builder()
            // this doesn't work when striding
            .prefilter(false)
            .build(patterns)?;

        let builder = nfa::contiguous::Builder::new();
        let matcher: Arc<dyn AcAutomaton> = match builder.build_from_noncontiguous(&nnfa) {
            Ok(cnfa) => Arc::new(cnfa),
            Err(_) => Arc::new(nnfa),
        };

        Ok(Self {
            matcher,
            decoder_mapping,
            unexpanded: unexpanded.into(),
        })
    }

    fn expand_patterns<I: IntoIterator<Item = P>, P: AsRef<str>>(
        patterns: I,
    ) -> (Vec<Box<[u8]>>, Vec<DecodingContext>) {
        let mut encoded_patterns = vec![];
        let mut decoders = vec![];

        for pattern in patterns.into_iter() {
            let pat = pattern.as_ref();

            for codec_generator in ALL_CODEC_GENERATORS {
                for codec in codec_generator(pat) {
                    let Codec {
                        encoded,
                        name,
                        decoder,
                        metadata,
                    } = codec;

                    // also generate the backwards version of the pattern
                    let mut reverse_encoded = Vec::from(encoded.clone());
                    reverse_encoded.reverse();

                    encoded_patterns.push(encoded);
                    decoders.push(DecodingContext {
                        decoder,
                        name,
                        metadata: metadata.clone(),
                        match_direction: MatchDirection::Forward,
                    });

                    encoded_patterns.push(Box::from(reverse_encoded));
                    decoders.push(DecodingContext {
                        decoder,
                        name,
                        metadata: metadata,
                        match_direction: MatchDirection::Backward,
                    });
                }
            }
        }

        assert_eq!(encoded_patterns.len(), decoders.len());
        (encoded_patterns, decoders)
    }

    fn find_iter<'a>(&self, haystack: Stride<'a, u8>) -> StridedFindOverlapping<'a> {
        StridedFindOverlapping::new(self.matcher.clone(), haystack)
            .expect("I fucked up creating the aho-corasick automaton, sorry...")
    }

    pub fn search<'a>(
        &self,
        haystack: Stride<'a, u8>,
    ) -> impl Iterator<Item = (String, DecoderName, MatchDirection)> {
        self.find_iter(haystack)
            .filter_map(move |match_| {
                // Look up the decoder for this match and whether it was a match going forwards or backwards
                let DecodingContext {
                    decoder,
                    name,
                    ref metadata,
                    match_direction,
                } = self.decoder_mapping[match_.pattern().as_usize()];

                // Extend the match in the correct direcion, i.e.
                // if we matched "flag{" going forwards then walk forwards to grab more of the input
                // to find the rest of the flag
                let to_decode = Self::expand_search(haystack, match_, match_direction);
                let decoded = decoder(to_decode, metadata.clone())?;
                Some((decoded, name, match_direction))
            })
            .map(|(decoded, name, direction)| {
                let flag = Self::postprocess_match(decoded);
                (flag, name, direction)
            })
            .filter(|(flag, _, _)| {
                // some codecs can produce erroneous matches which don't actually start with a flag
                // prefix, filter those out here
                self.unexpanded
                    .iter()
                    .any(|flag_prefix| flag.starts_with(flag_prefix))
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
        // 1. we truncated to utf8_valid_end and thus all the data before this is UTF8
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
    #[case("flag{gimme_the_whole_flag}", 1, &["flag{gimme_the_whole_flag}"], MatchDirection::Forward)]
    #[case("flag{short} with some garbage after", 1, &["flag{short}"], MatchDirection::Forward)]
    #[case("and some trash before flag{short}", 1, &["flag{short}"], MatchDirection::Forward)]
    #[case(b"\xFF\xFF\xFFflag{short}\xFF\xFF\xFF", 1, &["flag{short}"], MatchDirection::Forward)]
    #[case("}galf_elohw_eht_emmig{galf", 1, &["flag{gimme_the_whole_flag}"], MatchDirection::Backward)]
    #[case("}trohs{galf with some garbage after", 1, &["flag{short}"], MatchDirection::Backward)]
    #[case("and some trash before }trohs{galf", 1, &["flag{short}"], MatchDirection::Backward)]
    #[case(b"\xFF\xFF\xFF}trohs{galf\xFF\xFF\xFF", 1, &["flag{short}"], MatchDirection::Backward)]
    #[case("ffllaagg{{ffllaagg12}}", 2, &["flag{flag1}", "flag{flag2}"], MatchDirection::Forward)]
    #[case("ffffllaagg{{ffllaagg12}}", 2, &["flag{flag1}", "flag{flag2}"], MatchDirection::Forward)]
    #[case("ffflllaaaggg{{{ffflllaaaggg123}}}", 3, &["flag{flag1}", "flag{flag2}", "flag{flag3}"], MatchDirection::Forward)]
    #[case("}}}321gggaaalllfff{{{gggaaalllfff", 3, &["flag{flag3}", "flag{flag2}", "flag{flag1}"], MatchDirection::Backward)]
    #[case("fAAAlAAAaAAAgAAA{AAAfAAAlAAAaAAAgAAA1AAA}AAA", 4, &["flag{flag1}"], MatchDirection::Forward)]
    #[case("AAAAfAAAlAAAaAAAgAAA{AAAfAAAlAAAaAAAgAAA1AAA}AAA", 4, &["flag{flag1}"], MatchDirection::Forward)]
    fn test_identity_match(
        #[case] haystack: impl AsRef<[u8]>,
        #[case] stride_length: usize,
        #[case] correct: &[&str],
        #[case] correct_direction: MatchDirection,
    ) {
        let searcher = Searcher::new(["flag{"]).unwrap();

        let haystack = Stride::new(haystack.as_ref());
        let all_found: Vec<(String, DecoderName, MatchDirection)> = haystack
            .substrides(stride_length)
            .flat_map(|pile| searcher.search(pile))
            .collect();

        // with more codecs it turns out many codecs can decode the flags so just check that all of
        // the flags are found by the intended codec ignoring if other codecs find them
        for correct_flag in correct {
            let mut found = false;
            for (flag, decoder_name, match_direction) in &all_found {
                if flag == correct_flag
                    && *decoder_name == "UTF8"
                    && *match_direction == correct_direction
                {
                    found = true;
                    break;
                }
            }
            assert!(found, "Failed to find {correct_flag} in input")
        }
    }

    #[rstest]
    #[case(
        "ZmxhZ3tnaW1tZV90aGVfd2hvbGVfZmxhZ30=",
        // All three codecs are valid for this flag
        &[
            ("flag{gimme_the_whole_flag}", "base64"),
            ("flag{gimme_the_whole_flag}", "base64-URL-safe"),
            ("flag{gimme_the_whole_flag}", "base64-IMAP-modified-UTF7"),
        ],
    )]
    #[case("ZmxhZ3s-dWhTKiR9", 
        // All three codecs are valid for this flag, but only one is correct :)
        &[
            ("flag{", "base64"),
            ("flag{>uhS*$}", "base64-URL-safe"),
            ("flag{", "base64-IMAP-modified-UTF7"),
        ]
    )]
    #[case("MZWGCZ33M5UW23LFL52GQZK7O5UG63DFL5TGYYLHPU", 
        &[
            ("flag{gimme_the_whole_flag}", "base32-RFC-4648"),
        ]
    )]
    fn test_codecs(#[case] haystack: impl AsRef<[u8]>, #[case] correct: &[(&str, &str)]) {
        let searcher = Searcher::new(["flag{"]).unwrap();
        let haystack = Stride::new(haystack.as_ref());
        let found: Vec<_> = searcher.search(haystack).collect();
        assert_eq!(found.len(), correct.len(), "Found: {found:?}");

        for ((flag, decoder_name, match_direction), (correct, decoder)) in
            found.into_iter().zip(correct)
        {
            assert_eq!(&flag, correct);
            assert_eq!(&decoder_name, decoder);
            assert_eq!(match_direction, MatchDirection::Forward);
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

            let mut flag_found = false;
            for (flag, decoder_name, match_direction) in found.into_iter() {
                if decoder_name == "UTF8" {
                    assert_eq!(flag, correct);
                    assert_eq!(match_direction, MatchDirection::Forward);
                    flag_found = true;
                }
            }
            assert!(flag_found, "UTF8 codec failed to find flag");
        }
    }
}
