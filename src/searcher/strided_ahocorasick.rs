use std::sync::Arc;

use aho_corasick::{
    Anchored, Match, MatchError, Span,
    automaton::{Automaton, StateID},
};
use strided::Stride;

// A custom Input-style struct for our use case to trivially support different strides
pub struct Input<'h> {
    haystack: Stride<'h, u8>,
    span: Span,
}

impl<'h> Input<'h> {
    fn new(haystack: Stride<'h, u8>) -> Self {
        Self {
            haystack,
            span: Span {
                start: 0,
                end: haystack.len(),
            },
        }
    }

    fn haystack(&self) -> Stride<'h, u8> {
        self.haystack
    }

    fn set_start(&mut self, start: usize) {
        self.span.start = start;
    }

    fn start(&self) -> usize {
        self.span.start
    }

    fn end(&self) -> usize {
        self.span.end
    }

    fn is_done(&self) -> bool {
        self.span.start > self.span.end
    }
}

impl<'h> From<Stride<'h, u8>> for Input<'h> {
    fn from(haystack: Stride<'h, u8>) -> Self {
        Self::new(haystack)
    }
}

// Copied as much as possible from the FindIter implemention in the aho-corasick crate
pub struct StridedFindIter<'h> {
    aut: Arc<dyn Automaton>,
    input: Input<'h>,
    last_match_end: Option<usize>,
}

impl<'h> StridedFindIter<'h> {
    pub fn new(aut: Arc<dyn Automaton>, input: impl Into<Input<'h>>) -> Result<Self, MatchError> {
        let _ = aut.start_state(Anchored::No)?;
        Ok(Self {
            aut,
            input: input.into(),
            last_match_end: None,
        })
    }

    fn search(&self) -> Option<Match> {
        try_find_fwd(&self.aut, &self.input).expect("Already checked that no error can occur here")
    }

    #[cold]
    #[inline(never)]
    fn handle_overlapping_empty_match(&mut self, mut m: Match) -> Option<Match> {
        assert!(m.is_empty());
        if Some(m.end()) == self.last_match_end {
            self.input
                .set_start(self.input.start().checked_add(1).unwrap());
            m = self.search()?;
        }
        Some(m)
    }
}

impl<'h> Iterator for StridedFindIter<'h> {
    type Item = Match;

    #[inline(always)]
    fn next(&mut self) -> Option<Match> {
        let mut m = self.search()?;
        if m.is_empty() {
            m = self.handle_overlapping_empty_match(m)?;
        }
        self.input.set_start(m.end());
        self.last_match_end = Some(m.end());
        Some(m)
    }
}

#[inline(always)]
fn get_match(aut: &Arc<dyn Automaton>, sid: StateID, index: usize, at: usize) -> Match {
    let pid = aut.match_pattern(sid, index);
    let len = aut.pattern_len(pid);
    Match::new(pid, (at - len)..at)
}

fn try_find_fwd(aut: &Arc<dyn Automaton>, input: &Input<'_>) -> Result<Option<Match>, MatchError> {
    if input.is_done() {
        return Ok(None);
    }

    try_find_fwd_imp(aut, input)
}

#[inline(always)]
fn try_find_fwd_imp(
    aut: &Arc<dyn Automaton>,
    input: &Input<'_>,
) -> Result<Option<Match>, MatchError> {
    let mut sid = aut.start_state(Anchored::No)?;
    let mut at = input.start();
    let mut mat = None;
    if aut.is_match(sid) {
        mat = Some(get_match(aut, sid, 0, at));
        return Ok(mat);
    }

    while at < input.end() {
        sid = aut.next_state(Anchored::No, sid, input.haystack()[at]);
        if aut.is_special(sid) {
            if aut.is_dead(sid) {
                return Ok(mat);
            } else if aut.is_match(sid) {
                let m = get_match(aut, sid, 0, at + 1);
                mat = Some(m);
                return Ok(mat);
            } else {
                debug_assert!(false, "unreachable");
            }
        }
        at += 1;
    }

    Ok(mat)
}
