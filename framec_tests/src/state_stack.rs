//! Tests the basic functionality of the state stack feature. This test case
//! does not include any features that require a state context.

type Log = Vec<String>;
include!(concat!(env!("OUT_DIR"), "/", "state_stack.rs"));

impl StateStack {
    pub fn log(&mut self, msg: String) {
        self.tape.push(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that a pop restores a pushed state.
    fn push_pop() {
        let mut sm = StateStack::new();
        assert_eq!(sm.state, StateStackState::A);
        sm.push();
        sm.to_b();
        assert_eq!(sm.state, StateStackState::B);
        sm.pop();
        assert_eq!(sm.state, StateStackState::A);
    }

    #[test]
    /// Test that multiple states can be pushed and subsequently restored by
    /// pops, LIFO style.
    fn multiple_push_pops() {
        let mut sm = StateStack::new();
        assert_eq!(sm.state, StateStackState::A);
        sm.push();
        sm.to_c();
        sm.push();
        sm.to_a();
        sm.push();
        sm.push();
        sm.to_c(); // no push
        sm.to_b();
        sm.push();
        sm.to_c();
        sm.push(); // stack top-to-bottom: C, B, A, A, C, A
        sm.to_a();
        assert_eq!(sm.state, StateStackState::A);
        sm.pop();
        assert_eq!(sm.state, StateStackState::C);
        sm.to_a();
        assert_eq!(sm.state, StateStackState::A);
        sm.pop();
        assert_eq!(sm.state, StateStackState::B);
        sm.pop();
        assert_eq!(sm.state, StateStackState::A);
        sm.pop();
        assert_eq!(sm.state, StateStackState::A);
        sm.pop();
        assert_eq!(sm.state, StateStackState::C);
        sm.to_b();
        sm.push();
        sm.to_c();
        sm.push(); // stack top-to-bottom: C, B, A
        sm.to_a();
        sm.to_b();
        assert_eq!(sm.state, StateStackState::B);
        sm.pop();
        assert_eq!(sm.state, StateStackState::C);
        sm.pop();
        assert_eq!(sm.state, StateStackState::B);
        sm.pop();
        assert_eq!(sm.state, StateStackState::A);
    }

    #[test]
    /// Test that pop transitions trigger enter/exit events.
    fn pop_transition_events() {
        let mut sm = StateStack::new();
        sm.to_b();
        sm.push();
        sm.to_a();
        sm.push();
        sm.to_c();
        sm.push(); // stack top-to-bottom: C, A, B
        sm.to_a();
        sm.tape.clear();
        assert_eq!(sm.state, StateStackState::A);
        sm.pop();
        assert_eq!(sm.state, StateStackState::C);
        assert_eq!(sm.tape, vec!["A:<", "C:>"]);
        sm.tape.clear();
        sm.pop();
        sm.pop();
        assert_eq!(sm.state, StateStackState::B);
        assert_eq!(sm.tape, vec!["C:<", "A:>", "A:<", "B:>"]);
    }

    #[test]
    /// Test that pop change-states do not trigger enter/exit events.
    fn pop_change_state_no_events() {
        let mut sm = StateStack::new();
        sm.to_b();
        sm.push();
        sm.to_a();
        sm.push();
        sm.to_c();
        sm.push(); // stack top-to-bottom: C, A, B
        sm.to_a();
        sm.tape.clear();
        assert_eq!(sm.state, StateStackState::A);
        sm.pop_change();
        assert_eq!(sm.state, StateStackState::C);
        assert!(sm.tape.is_empty());
        sm.pop();
        sm.pop_change();
        assert_eq!(sm.state, StateStackState::B);
        assert_eq!(sm.tape, vec!["C:<", "A:>"]);
    }
}