mod regex;

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

pub use regex::Error as RegexError;
use regex::{RegexAtom, RegexJoin, RegexNode, RegexOr, RegexRepeat};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn compile_regex(input: &str) -> Result<StateMachines, String> {
    let nfa = Nfa::from_regex(input).map_err(|e| e.to_string())?;
    let original_dfa = Dfa::from_nfa(&nfa);
    let dfa = Nfa::from(original_dfa);

    Ok(StateMachines { nfa, dfa })
}

#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct StateMachines {
    nfa: Nfa,
    dfa: Nfa,
}

#[derive(Debug, Serialize, Tsify)]
pub struct Nfa {
    pub states: Vec<NfaState>,
}

#[derive(Debug, Clone, Serialize, Tsify)]
pub struct NfaState {
    pub branches: BTreeMap<char, Vec<usize>>,
    pub epsilon_transitions: Vec<usize>,
    pub accepts: bool,
}

impl Nfa {
    pub fn from_regex(regex: &str) -> Result<Self, RegexError> {
        let regex: regex::Regex = regex.parse()?;
        let mut machine = Nfa { states: Vec::new() };
        let initial_state = machine.alloc_state();
        let final_state = machine.insert(initial_state, &regex.root);
        machine.states[final_state].accepts = true;
        Ok(machine)
    }

    fn alloc_state(&mut self) -> usize {
        let id = self.states.len();
        self.states.push(NfaState {
            branches: BTreeMap::new(),
            epsilon_transitions: Vec::new(),
            accepts: false,
        });
        id
    }

    fn insert(&mut self, state: usize, pattern: &RegexNode) -> usize {
        match pattern {
            RegexNode::Atom(e) => self.insert_atom(state, e),
            RegexNode::Repeat(e) => self.insert_repeat(state, e),
            RegexNode::Or(e) => self.insert_or(state, e),
            RegexNode::Join(e) => self.insert_join(state, e),
        }
    }

    fn insert_atom(&mut self, mut state: usize, pattern: &RegexAtom) -> usize {
        for c in pattern.literal.chars() {
            let s = self.alloc_state();
            let edges = self.states[state].branches.entry(c).or_default();
            edges.push(s);
            state = s;
        }
        state
    }

    fn insert_repeat(&mut self, state: usize, pattern: &RegexRepeat) -> usize {
        let loop_start = self.alloc_state();
        self.states[state].epsilon_transitions.push(loop_start);
        let s = self.insert(loop_start, &pattern.pattern);
        self.states[s].epsilon_transitions.push(loop_start);
        loop_start
    }

    fn insert_or(&mut self, state: usize, pattern: &RegexOr) -> usize {
        let s0 = self.insert(state, &pattern.left);
        let s1 = self.insert(state, &pattern.right);

        let s = self.alloc_state();

        self.states[s0].epsilon_transitions.push(s);
        self.states[s1].epsilon_transitions.push(s);

        s
    }

    fn insert_join(&mut self, state: usize, pattern: &RegexJoin) -> usize {
        let state = self.insert(state, &pattern.left);
        self.insert(state, &pattern.right)
    }
}

impl From<Dfa> for Nfa {
    fn from(value: Dfa) -> Self {
        let states = value
            .states
            .into_iter()
            .map(|s| NfaState {
                branches: s.branches.into_iter().map(|(k, v)| (k, vec![v])).collect(),
                epsilon_transitions: Vec::new(),
                accepts: s.accepts,
            })
            .collect();

        Nfa { states }
    }
}

#[derive(Debug, Clone, Serialize, Tsify)]
pub struct Dfa {
    pub states: Vec<DfaState>,
}

#[derive(Debug, Clone, Serialize, Tsify)]
pub struct DfaState {
    pub branches: BTreeMap<char, usize>,
    pub accepts: bool,
}

impl Dfa {
    pub fn from_nfa(state_machine: &Nfa) -> Self {
        let mut states = Vec::new();
        let mut state_map = BTreeMap::new();

        let initial_state = epsilon_closure(state_machine, [0]);
        states.push(DfaState {
            branches: BTreeMap::new(),
            accepts: initial_state
                .iter()
                .any(|&s| state_machine.states[s].accepts),
        });
        state_map.insert(initial_state.clone(), 0);

        let mut unchecked_states = VecDeque::from_iter([initial_state]);

        while let Some(s) = unchecked_states.pop_front() {
            let &state_id = state_map.get(&s).unwrap();
            let mut branches = BTreeMap::new();
            let transisions = transitions(state_machine, s.iter().copied());

            for (c, s) in transisions {
                let id = match state_map.get(&s) {
                    Some(s) => *s,
                    None => {
                        let id = states.len();
                        states.push(DfaState {
                            branches: BTreeMap::new(),
                            accepts: s.iter().any(|&s| state_machine.states[s].accepts),
                        });

                        state_map.insert(s.clone(), id);
                        unchecked_states.push_back(s);
                        id
                    }
                };

                branches.insert(c, id);
            }

            states[state_id].branches = branches;
        }

        Self { states }
    }

    pub fn optimize(&self) -> Self {
        let _reachables = reachable_states(self, 0);

        todo!()
    }
}

fn reachable_states(dfa: &Dfa, state: usize) -> BTreeSet<usize> {
    let mut reachables = BTreeSet::from_iter([state]);
    let mut unchecked_states = vec![state];

    while let Some(state) = unchecked_states.pop() {
        for &s in dfa.states[state].branches.values() {
            if reachables.contains(&s) {
                continue;
            }
            reachables.insert(s);
            unchecked_states.push(s);
        }
    }

    reachables
}

fn transitions(
    state_machine: &Nfa,
    states: impl IntoIterator<Item = usize>,
) -> BTreeMap<char, BTreeSet<usize>> {
    let mut transisions = BTreeMap::new();

    let pairs = states
        .into_iter()
        .flat_map(|s| state_machine.states[s].branches.iter());

    for (&c, s) in pairs {
        let v: &mut BTreeSet<usize> = transisions.entry(c).or_default();
        v.extend(s);
    }

    for s in transisions.values_mut() {
        *s = epsilon_closure(state_machine, s.iter().copied());
    }

    transisions
}

fn epsilon_closure(
    state_machine: &Nfa,
    states: impl IntoIterator<Item = usize>,
) -> BTreeSet<usize> {
    let mut unchecked: Vec<_> = states.into_iter().collect();
    let mut reachable = BTreeSet::new();

    while let Some(s) = unchecked.pop() {
        if !reachable.insert(s) {
            continue;
        }

        unchecked.extend_from_slice(&state_machine.states[s].epsilon_transitions);
    }

    reachable
}
