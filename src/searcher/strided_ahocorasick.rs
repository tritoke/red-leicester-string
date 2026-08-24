use std::sync::Arc;

use aho_corasick::{
    Anchored, Match, MatchError, MatchKind, Span,
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

    fn get_anchored(&self) -> Anchored {
        Anchored::No
    }
}

impl<'h> From<Stride<'h, u8>> for Input<'h> {
    fn from(haystack: Stride<'h, u8>) -> Self {
        Self::new(haystack)
    }
}

// Copied as much as possible from the FindIter implemention in the aho-corasick crate
pub struct StridedFindOverlapping<'h> {
    aut: Arc<dyn Automaton>,
    input: Input<'h>,
    state: OverlappingState,
}

struct OverlappingState {
    /// The match reported by the most recent overlapping search to use this
    /// state.
    ///
    /// If a search does not find any matches, then it is expected to clear
    /// this value.
    mat: Option<Match>,
    /// The state ID of the state at which the search was in when the call
    /// terminated. When this is a match state, `last_match` must be set to a
    /// non-None value.
    ///
    /// A `None` value indicates the start state of the corresponding
    /// automaton. We cannot use the actual ID, since any one automaton may
    /// have many start states, and which one is in use depends on search-time
    /// factors (such as whether the search is anchored or not).
    id: Option<StateID>,
    /// The position of the search.
    ///
    /// When `id` is None (i.e., we are starting a search), this is set to
    /// the beginning of the search as given by the caller regardless of its
    /// current value. Subsequent calls to an overlapping search pick up at
    /// this offset.
    at: usize,
    /// The index into the matching patterns of the next match to report if the
    /// current state is a match state. Note that this may be 1 greater than
    /// the total number of matches to report for the current match state. (In
    /// which case, no more matches should be reported at the current position
    /// and the search should advance to the next position.)
    next_match_index: Option<usize>,
}

impl OverlappingState {
    /// Create a new overlapping state that begins at the start state.
    pub fn start() -> OverlappingState {
        OverlappingState {
            mat: None,
            id: None,
            at: 0,
            next_match_index: None,
        }
    }

    /// Return the match result of the most recent search to execute with this
    /// state.
    ///
    /// Every search will clear this result automatically, such that if no
    /// match is found, this will always correctly report `None`.
    pub fn get_match(&self) -> Option<Match> {
        self.mat
    }
}

impl<'h> StridedFindOverlapping<'h> {
    pub fn new(aut: Arc<dyn Automaton>, input: impl Into<Input<'h>>) -> Result<Self, MatchError> {
        if !matches!(aut.match_kind(), MatchKind::Standard) {
            return Err(MatchError::unsupported_overlapping(aut.match_kind()));
        }

        let input = input.into();
        let _ = aut.start_state(input.get_anchored())?;
        let state = OverlappingState::start();
        Ok(Self { aut, input, state })
    }

    fn try_find_overlapping(&mut self) {
        try_find_overlapping_fwd(&self.aut, &self.input, &mut self.state)
            .expect("Already checked that no error can occur here")
    }
}

impl<'h> Iterator for StridedFindOverlapping<'h> {
    type Item = Match;

    #[inline(always)]
    fn next(&mut self) -> Option<Match> {
        self.try_find_overlapping();
        self.state.get_match()
    }
}

#[inline(always)]
fn get_match(aut: &Arc<dyn Automaton>, sid: StateID, index: usize, at: usize) -> Match {
    let pid = aut.match_pattern(sid, index);
    let len = aut.pattern_len(pid);
    Match::new(pid, (at - len)..at)
}

fn try_find_overlapping_fwd(
    aut: &Arc<dyn Automaton>,
    input: &Input<'_>,
    state: &mut OverlappingState,
) -> Result<(), MatchError> {
    state.mat = None;
    if input.is_done() {
        return Ok(());
    }

    try_find_overlapping_fwd_imp(aut, input, state)
}

#[inline(always)]
fn try_find_overlapping_fwd_imp(
    aut: &Arc<dyn Automaton>,
    input: &Input<'_>,
    state: &mut OverlappingState,
) -> Result<(), MatchError> {
    let mut sid = match state.id {
        None => {
            let sid = aut.start_state(input.get_anchored())?;
            // Handle the case where the start state is a match state. That is,
            // the empty string is in our automaton. We report every match we
            // can here before moving on and updating 'state.at' and 'state.id'
            // to find more matches in other parts of the haystack.
            if aut.is_match(sid) {
                let i = state.next_match_index.unwrap_or(0);
                let len = aut.match_len(sid);
                if i < len {
                    state.next_match_index = Some(i + 1);
                    state.mat = Some(get_match(aut, sid, i, input.start()));
                    return Ok(());
                }
            }
            state.at = input.start();
            state.id = Some(sid);
            state.next_match_index = None;
            state.mat = None;
            sid
        }
        Some(sid) => {
            // If we still have matches left to report in this state then
            // report them until we've exhausted them. Only after that do we
            // advance to the next offset in the haystack.
            if let Some(i) = state.next_match_index {
                let len = aut.match_len(sid);
                if i < len {
                    state.next_match_index = Some(i + 1);
                    state.mat = Some(get_match(aut, sid, i, state.at + 1));
                    return Ok(());
                }
                // Once we've reported all matches at a given position, we need
                // to advance the search to the next position.
                state.at += 1;
                state.next_match_index = None;
                state.mat = None;
            }
            sid
        }
    };
    while state.at < input.end() {
        sid = aut.next_state(input.get_anchored(), sid, input.haystack()[state.at]);
        if aut.is_special(sid) {
            state.id = Some(sid);
            if aut.is_dead(sid) {
                return Ok(());
            } else if aut.is_match(sid) {
                state.next_match_index = Some(1);
                state.mat = Some(get_match(aut, sid, 0, state.at + 1));
                return Ok(());
            } else {
                // When pre.is_none(), then starting states should not be
                // treated as special. That is, without a prefilter, is_special
                // should only return true when the state is a dead or a match
                // state.
                //
                // ... except for one special case: in stream searching, we
                // currently call overlapping search with a 'None' prefilter,
                // regardless of whether one exists or not, because stream
                // searching can't currently deal with prefilters correctly in
                // all cases.
            }
        }
        state.at += 1;
    }
    state.id = Some(sid);
    Ok(())
}
