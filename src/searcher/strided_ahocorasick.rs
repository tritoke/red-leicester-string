use std::sync::Arc;

use aho_corasick::{
    Anchored, Input, Match, MatchError,
    automaton::{Automaton, StateID},
};

// Copied as much as possible from the FindIter implemention in the aho-corasick crate
pub struct StridedFindIter<'h> {
    aut: Arc<dyn Automaton>,
    input: Input<'h>,
    last_match_end: Option<usize>,
    stride: usize,
}

impl<'h> StridedFindIter<'h> {
    pub fn new(
        aut: Arc<dyn Automaton>,
        input: Input<'h>,
        stride: usize,
    ) -> Result<Self, MatchError> {
        let _ = aut.start_state(input.get_anchored())?;
        Ok(Self {
            aut,
            input,
            last_match_end: None,
            stride,
        })
    }

    fn search(&self) -> Option<Match> {
        try_find_fwd(&self.aut, &self.input, self.stride)
            .expect("Already checked that no error can occur here")
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
fn get_match(
    aut: &Arc<dyn Automaton>,
    sid: StateID,
    index: usize,
    at: usize,
    stride: usize,
) -> Match {
    let pid = aut.match_pattern(sid, index);
    let len = aut.pattern_len(pid);
    Match::new(pid, at - (len - 1) * stride..at + 1)
}

fn try_find_fwd(
    aut: &Arc<dyn Automaton>,
    input: &Input<'_>,
    stride: usize,
) -> Result<Option<Match>, MatchError> {
    if input.is_done() {
        return Ok(None);
    }

    try_find_fwd_imp(aut, input, stride)
}

#[inline(always)]
fn try_find_fwd_imp(
    aut: &Arc<dyn Automaton>,
    input: &Input<'_>,
    stride: usize,
) -> Result<Option<Match>, MatchError> {
    let mut sid = aut.start_state(input.get_anchored())?;
    let mut at = input.start();
    let mut mat = None;
    if aut.is_match(sid) {
        mat = Some(get_match(aut, sid, 0, at, stride));
        return Ok(mat);
    }

    while at < input.end() {
        sid = aut.next_state(Anchored::No, sid, input.haystack()[at]);
        if aut.is_special(sid) {
            if aut.is_dead(sid) {
                return Ok(mat);
            } else if aut.is_match(sid) {
                let m = get_match(aut, sid, 0, at, stride);
                mat = Some(m);
                return Ok(mat);
            } else {
                debug_assert!(false, "unreachable");
            }
        }
        at += stride;
    }

    Ok(mat)
}
