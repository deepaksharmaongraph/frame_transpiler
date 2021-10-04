//! This module defines a generic representation of a transition event within
//! a running state machine.

use crate::environment::Environment;
use crate::state::State;
use std::cell::Ref;

/// Was this a standard transition or a change-state transition, which bypasses
/// enter/exit events?
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum TransitionKind {
    ChangeState,
    Transition,
}

/// Information about a transition or change-state event.
pub struct TransitionInfo<'a> {
    /// What kind of transition occurred?
    pub kind: TransitionKind,

    /// The state before the transition.
    pub old_state: Ref<'a, dyn State>,

    /// The state after the transition.
    pub new_state: Ref<'a, dyn State>,

    /// Arguments to the exit handler of the old state.
    pub exit_arguments: &'a dyn Environment,

    /// Arguments to the enter handler of the new state.
    pub enter_arguments: &'a dyn Environment,
}