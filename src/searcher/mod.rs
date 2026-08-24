use std::{borrow::Cow, sync::Arc};

use aho_corasick::{Match, automaton::Automaton, nfa};

mod codecs;
use codecs::{ALL_CODECS, Decoder, DecoderName};

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

    fn find_with_stride<'a>(&self, haystack: &'a [u8], stride: usize) -> StridedFindIter<'a> {
        StridedFindIter::new(self.matcher.clone(), haystack.into(), stride)
            .expect("I fucked up creating the aho-corasick automaton, sorry...")
    }

    pub fn search<'a>(
        &self,
        haystack: &'a [u8],
        stride: usize,
    ) -> impl Iterator<Item = (Cow<'a, str>, DecoderName, MatchDirection)> {
        self.find_with_stride(haystack, stride)
            .filter_map(move |match_| {
                dbg!(match_);
                // Look up the decoder for this match and whether it was a match going forwards or backwards
                let (decoder, name, direction) = self.decoders[match_.pattern().as_usize()];

                // Extend the match in the correct direcion, i.e.
                // if we matched "flag{" going forwards then walk forwards to grab more of the input
                // to find the rest of the flag
                let to_decode = Self::expand_search(haystack, match_, direction, stride);
                let decoded = decoder(to_decode)?;
                Some((decoded, name, direction))
            })
            .map(|(decoded, name, direction)| {
                let flag = Self::postprocess_match(decoded);
                (flag, name, direction)
            })
    }

    fn expand_search<'a>(
        haystack: &'a [u8],
        match_: Match,
        direction: MatchDirection,
        stride: usize,
    ) -> Cow<'a, [u8]> {
        match direction {
            MatchDirection::Forward if stride == 1 => {
                // going forwards with stride=1 we can simply borrow from the haystack
                let raw_match_data = &haystack
                    [match_.start()..usize::min(haystack.len(), match_.end() + MAX_FLAG_LENGTH)];
                Cow::Borrowed(raw_match_data)
            }
            MatchDirection::Forward => {
                let raw_match_data = &haystack[match_.start()
                    ..usize::min(haystack.len(), match_.end() + MAX_FLAG_LENGTH * stride)];
                eprintln!("{match_:?}");
                eprintln!(
                    "{:?}",
                    match_.start()
                        ..usize::min(haystack.len(), match_.end() + MAX_FLAG_LENGTH * stride)
                );
                let to_decode = raw_match_data.iter().copied().step_by(stride).collect();
                Cow::Owned(to_decode)
            }
            MatchDirection::Backward => {
                // going forwards we can need to find the candidate flag area, then reverse it
                // in order to use the decoder
                let raw_match_data = &haystack
                    [match_.start().saturating_sub(MAX_FLAG_LENGTH * stride)..match_.end()];
                let mut reversed = if stride == 1 {
                    raw_match_data.to_owned()
                } else {
                    raw_match_data.iter().copied().step_by(stride).collect()
                };
                reversed.reverse();
                Cow::Owned(reversed)
            }
        }
    }

    fn postprocess_match<'a>(extended_match_data: Cow<'a, [u8]>) -> Cow<'a, str> {
        // compute where is valid UTF8 / where the closing brace is via as_ref so we can the work is
        // shared for each branch of the Cow's state
        let haystack = extended_match_data.as_ref();
        let utf8_valid_end = encoding_rs::Encoding::utf8_valid_up_to(haystack);
        let closing_pos = memchr::memchr(CLOSING_CHAR, &haystack).unwrap_or(utf8_valid_end);

        // closing_pos is incremented as utf8_valid_end is an index valid for [..<to>] style
        // indexing but closing_pos is valid for [..=<to>] style indexing and this lets us use both
        // in the same place
        let truncate_to = usize::min(utf8_valid_end, closing_pos + 1);

        match extended_match_data {
            Cow::Borrowed(borrow) => {
                // SAFETY: borrow must contain only valid UTF8 data as either:
                // 1. we truncated to utf8_valid_end and thus all the data before this is utf8
                // 2. we hit closing_pos, which is strictly shorter than utf8_valid_end and is a
                //    unicode codepoint boundary and is thus safe to truncate to.
                let flag: &'a str =
                    unsafe { std::str::from_utf8_unchecked(&borrow[..truncate_to]) };

                Cow::Borrowed(&flag)
            }
            Cow::Owned(mut owned) => {
                // discard the invalid UTF8 data or until we see a closing brace
                owned.resize(truncate_to, 0);

                // not needed but we might as well I think
                owned.shrink_to_fit();

                // SAFETY: owned must contain only valid UTF8 data as either:
                // 1. we truncated to utf8_valid_end and thus all the data before this is utf8
                // 2. we hit closing_pos, which is strictly shorter than utf8_valid_end and is a
                //    unicode codepoint boundary and is thus safe to truncate to.
                let flag = unsafe { String::from_utf8_unchecked(owned) };
                Cow::Owned(flag)
            }
        }
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
    #[case("flag{", "flag{gimme_the_whole_flag}", "flag{gimme_the_whole_flag}")]
    #[case("flag{", "flag{short} with some garbage after", "flag{short}")]
    #[case("flag{", "and some trash before flag{short}", "flag{short}")]
    #[case("flag{", b"\xFF\xFF\xFFflag{short}\xFF\xFF\xFF", "flag{short}")]
    fn test_identity_match_forward(
        #[case] pattern: impl AsRef<[u8]>,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] correct: &str,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let found: Vec<(Cow<'_, str>, DecoderName, MatchDirection)> =
            searcher.search(haystack.as_ref(), 1).collect();
        assert_eq!(found.len(), 1);

        let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
        assert_eq!(flag, correct);
        assert_eq!(decoder_name, "UTF8");
        assert_eq!(match_direction, MatchDirection::Forward);
    }

    #[rstest]
    #[case("flag{", "}galf_elohw_eht_emmig{galf", "flag{gimme_the_whole_flag}")]
    #[case("flag{", "}trohs{galf with some garbage after", "flag{short}")]
    #[case("flag{", "and some trash before }trohs{galf", "flag{short}")]
    #[case("flag{", b"\xFF\xFF\xFF}trohs{galf\xFF\xFF\xFF", "flag{short}")]
    fn test_identity_match_backwards(
        #[case] pattern: impl AsRef<[u8]>,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] correct: &str,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let found: Vec<(Cow<'_, str>, DecoderName, MatchDirection)> =
            searcher.search(haystack.as_ref(), 1).collect();
        assert_eq!(found.len(), 1);

        let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
        assert_eq!(flag, correct);
        assert_eq!(decoder_name, "UTF8");
        assert_eq!(match_direction, MatchDirection::Backward);
    }

    #[rstest]
    #[case("flag{", "ffllaagg{{ffllaagg12}}", 0, "flag{flag1}")]
    #[case("flag{", "ffllaagg{{ffllaagg12}}", 1, "flag{flag2}")]
    #[case("flag{", "ffffllaagg{{ffllaagg12}}", 0, "flag{flag1}")]
    #[case("flag{", "ffffllaagg{{ffllaagg12}}", 1, "flag{flag2}")]
    fn test_identity_match_stride_2(
        #[case] pattern: impl AsRef<[u8]>,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] start: usize,
        #[case] correct: &str,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let found: Vec<(Cow<'_, str>, DecoderName, MatchDirection)> =
            searcher.search(&haystack.as_ref()[start..], 2).collect();
        assert_eq!(found.len(), 1);

        let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
        assert_eq!(flag, correct);
        assert_eq!(decoder_name, "UTF8");
        assert_eq!(match_direction, MatchDirection::Forward);
    }

    #[rstest]
    #[case("flag{", "ffflllaaaggg{{{ffflllaaaggg123}}}", 0, "flag{flag1}")]
    #[case("flag{", "ffflllaaaggg{{{ffflllaaaggg123}}}", 1, "flag{flag2}")]
    #[case("flag{", "ffflllaaaggg{{{ffflllaaaggg123}}}", 2, "flag{flag3}")]
    fn test_identity_match_stride_3(
        #[case] pattern: impl AsRef<[u8]>,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] start: usize,
        #[case] correct: &str,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let found: Vec<(Cow<'_, str>, DecoderName, MatchDirection)> =
            searcher.search(&haystack.as_ref()[start..], 3).collect();
        assert_eq!(found.len(), 1);

        let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
        assert_eq!(flag, correct);
        assert_eq!(decoder_name, "UTF8");
        assert_eq!(match_direction, MatchDirection::Forward);
    }

    #[rstest]
    #[case("flag{", "}}}321gggaaalllfff{{{gggaaalllfff", 0, "flag{flag3}")]
    #[case("flag{", "}}}321gggaaalllfff{{{gggaaalllfff", 1, "flag{flag2}")]
    #[case("flag{", "}}}321gggaaalllfff{{{gggaaalllfff", 2, "flag{flag1}")]
    fn test_identity_match_stride_3_backwards(
        #[case] pattern: impl AsRef<[u8]>,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] start: usize,
        #[case] correct: &str,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let found: Vec<(Cow<'_, str>, DecoderName, MatchDirection)> =
            searcher.search(&haystack.as_ref()[start..], 3).collect();
        assert_eq!(found.len(), 1);

        let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
        assert_eq!(flag, correct);
        assert_eq!(decoder_name, "UTF8");
        assert_eq!(match_direction, MatchDirection::Backward);
    }

    #[rstest]
    #[case(
        "flag{",
        "fAAAlAAAaAAAgAAA{AAAfAAAlAAAaAAAgAAA1AAA}AAA",
        4,
        "flag{flag1}"
    )]
    #[case(
        "flag{",
        "AAAAfAAAlAAAaAAAgAAA{AAAfAAAlAAAaAAAgAAA1AAA}AAA",
        4,
        "flag{flag1}"
    )]
    fn test_identity_match_stride_large(
        #[case] pattern: impl AsRef<[u8]>,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] stride: usize,
        #[case] correct: &str,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let found: Vec<(Cow<'_, str>, DecoderName, MatchDirection)> =
            searcher.search(&haystack.as_ref(), stride).collect();
        assert_eq!(found.len(), 1);

        let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
        assert_eq!(flag, correct);
        assert_eq!(decoder_name, "UTF8");
        assert_eq!(match_direction, MatchDirection::Forward);
    }

    // This test is kinda slow in debug mode so only run in release mode tests
    #[cfg_attr(debug_assertions, allow(dead_code))]
    #[cfg_attr(not(debug_assertions), test)]
    fn test_daft() {
        let mut buf = Vec::with_capacity(100_000);
        let mut rng = rand::rng();
        buf.resize(buf.capacity(), 0);

        let correct = "flag{big_stripe}";
        for stride in 2..(buf.len() / 2) / correct.len() {
            rng.fill_bytes(&mut buf[..]);

            let start = rng.random_range(0..buf.len() / 2);
            for (i, c) in correct.as_bytes().iter().enumerate() {
                buf[start + i * stride] = *c;
            }

            let searcher = Searcher::new(["flag{"]).unwrap();

            let found: Vec<_> = searcher.search(&buf[start % stride..], stride).collect();
            assert_eq!(found.len(), 1);

            let (flag, decoder_name, match_direction) = found.into_iter().next().unwrap();
            assert_eq!(flag, correct);
            assert_eq!(decoder_name, "UTF8");
            assert_eq!(match_direction, MatchDirection::Forward);
        }
    }
}
