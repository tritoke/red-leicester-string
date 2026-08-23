use std::{borrow::Cow, sync::Arc};

use aho_corasick::{Input, Match, automaton::Automaton, nfa};

use crate::codecs::{ALL_CODECS, Decoder, DecoderName};

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
        let nnfa = nfa::noncontiguous::NFA::new(patterns)?;

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

    pub fn search<'a>(
        &self,
        haystack: &'a [u8],
    ) -> impl Iterator<Item = (Cow<'a, str>, DecoderName, MatchDirection)> {
        todo!()

        // self.matcher
        //     .find_overlapping_iter(haystack)
        //     .filter_map(|match_| {
        //         let (decoder, name, direction) = self.decoders[match_.pattern().as_usize()];
        //         let to_decode = Self::expand_search(haystack, match_, direction);
        //         let decoded = decoder(to_decode)?;
        //         Some((decoded, name, direction))
        //     })
        //     .map(|(decoded, name, direction)| {
        //         let flag = Self::postprocess_match(decoded);
        //         (flag, name, direction)
        //     })
    }

    fn expand_search<'a>(
        haystack: &'a [u8],
        match_: Match,
        direction: MatchDirection,
    ) -> Cow<'a, [u8]> {
        match direction {
            MatchDirection::Forward => {
                // going forwards we can simply borrow from the haystack
                let to_decode = &haystack
                    [match_.start()..usize::min(haystack.len(), match_.end() + MAX_FLAG_LENGTH)];
                Cow::Borrowed(to_decode)
            }
            MatchDirection::Backward => {
                // going forwards we can need to find the candidate flag area, then reverse it
                // in order to use the decoder
                let to_decode =
                    &haystack[match_.start().saturating_sub(MAX_FLAG_LENGTH)..match_.end()];
                let mut reversed = to_decode.to_owned();
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
    use rstest::rstest;

    #[rstest]
    #[case("flag{", "flag{gimme_the_whole_flag}", "flag{gimme_the_whole_flag}")]
    #[case("flag{", "flag{short} with some garbage after", "flag{short}")]
    #[case("flag{", "and some trash before flag{short}", "flag{short}")]
    #[case("flag{", b"\xFF\xFF\xFFflag{short}\xFF\xFF\xFF", "flag{short}")]
    fn test_identity_match(
        #[case] pattern: impl AsRef<[u8]>,
        #[case] haystack: impl AsRef<[u8]>,
        #[case] correct: impl AsRef<[u8]>,
    ) {
        let searcher = Searcher::new([pattern]).unwrap();

        let found: Vec<(Cow<'_, str>, DecoderName)> = searcher.search(haystack.as_ref()).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0.as_bytes(), correct.as_ref());
        assert_eq!(found[0].1, "UTF8");
    }
}
