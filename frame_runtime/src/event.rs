//! The event monitor maintains a history of previous Frame events and transitions, and enables
//! registering callbacks that will be automatically invoked whenever an event or transition occurs
//! in a running state machine.

use crate::live::*;
use std::collections::VecDeque;
use std::rc::Rc;

/// A trait alias for functions that take a method instance as an argument. Used as the type of
/// Frame event notification callbacks.
pub trait EventCallback<'a>: FnMut(Rc<dyn MethodInstance>) + Send + 'a {}
impl<'a, F> EventCallback<'a> for F where F: FnMut(Rc<dyn MethodInstance>) + Send + 'a {}

/// A trait alias for functions that take a transition instance as an argument. Used as the type of
/// state transition notification callbacks.
pub trait TransitionCallback<'a>: FnMut(&TransitionInstance) + Send + 'a {}
impl<'a, F> TransitionCallback<'a> for F where F: FnMut(&TransitionInstance) + Send + 'a {}

/// The event monitor.
pub struct EventMonitor<'a> {
    event_history_capacity: Option<usize>,
    transition_history_capacity: Option<usize>,
    event_history: VecDeque<Rc<dyn MethodInstance>>,
    transition_history: VecDeque<TransitionInstance>,
    event_sent_callbacks: Vec<Box<dyn EventCallback<'a>>>,
    event_handled_callbacks: Vec<Box<dyn EventCallback<'a>>>,
    transition_callbacks: Vec<Box<dyn TransitionCallback<'a>>>,
    // event_callbacks: Vec<Box<dyn FnMut(Rc<dyn MethodInstance>) + Send + 'a>>,
    // transition_callbacks: Vec<Box<dyn FnMut(&TransitionInstance) + Send + 'a>>,
}

impl<'a> EventMonitor<'a> {
    /// Create a new event monitor. The arguments indicate the number of events and transitions to
    /// maintain as history.
    pub fn new(event_capacity: Option<usize>, transition_capacity: Option<usize>) -> Self {
        EventMonitor {
            event_history_capacity: event_capacity,
            transition_history_capacity: transition_capacity,
            event_history: new_deque(&event_capacity),
            transition_history: new_deque(&transition_capacity),
            event_sent_callbacks: Vec::new(),
            event_handled_callbacks: Vec::new(),
            transition_callbacks: Vec::new(),
        }
    }

    /// Register a callback to be invoked when an event is sent, but before it has been handled.
    /// Use this when you want the notification order for events to reflect the order that the
    /// events are triggered, but don't care about the return value of handled events.
    ///
    /// Note that when an event triggers a transition, callbacks will be invoked for the related
    /// events in the following order:
    ///
    ///  * triggering event
    ///  * exit event for the old state, if any
    ///  * enter event for the new state, if any
    ///
    /// Note that the argument type for this function is `impl EventCallback<'a>`, but the trait
    /// alias is inlined to help Rust infer the argument type when callbacks are defined
    /// anonymously.
    pub fn add_event_sent_callback(
        &mut self,
        callback: impl FnMut(Rc<dyn MethodInstance>) + Send + 'a,
        // callback: impl EventCallback<'a>,
    ) {
        self.event_sent_callbacks.push(Box::new(callback));
    }

    /// Register a callback to be invoked after an event has been *completely* handled. Use this
    /// when you want the method instance argument to contain the return value of the event, if
    /// any.
    ///
    /// Note that when an event triggers a transition, callbacks will be invoked for the related
    /// events in the following order:
    ///
    ///  * exit event for the old state, if any
    ///  * enter event for the new state, if any
    ///  * triggering event
    ///
    /// Note that the argument type for this function is `impl EventCallback<'a>`, but the trait
    /// alias is inlined to help Rust infer the argument type when callbacks are defined
    /// anonymously.
    pub fn add_event_handled_callback(
        &mut self,
        callback: impl FnMut(Rc<dyn MethodInstance>) + Send + 'a,
        // callback: impl EventCallback<'a>,
    ) {
        self.event_handled_callbacks.push(Box::new(callback));
    }

    /// Register a callback to be called on each transition. Callbacks will be invoked after each
    /// transition completes, including the processing of exit and enter events.
    ///
    /// Note that the argument type for this function is `impl TransitionCallback<'a>`, but the
    /// trait alias is inlined to help Rust infer the argument type when callbacks are defined
    /// anonymously.
    pub fn add_transition_callback(
        &mut self,
        callback: impl FnMut(&TransitionInstance) + Send + 'a,
        // callback: impl TransitionCallback<'a>,
    ) {
        self.transition_callbacks.push(Box::new(callback));
    }

    /// Invoke the event-sent callbacks. This event will not be added to the history until the
    /// event has been completely handled. Clients shouldn't need to call this method. It will be
    /// called by code generated by Framec.
    pub fn event_sent(&mut self, event: Rc<dyn MethodInstance>) {
        for c in &mut self.event_sent_callbacks {
            (**c)(event.clone());
        }
    }

    /// Track that a Frame event was handled, calling any relevant callbacks and saving it to the
    /// history. Clients shouldn't need to call this method. It will be called by code generated by
    /// Framec.
    pub fn event_handled(&mut self, event: Rc<dyn MethodInstance>) {
        push_to_deque(
            &self.event_history_capacity,
            &mut self.event_history,
            event.clone(),
        );
        for c in &mut self.event_handled_callbacks {
            (**c)(event.clone());
        }
    }

    /// Track that a transition occurred with the provided arguments, calling all of the transition
    /// callbacks and saving it to the history. Clients shouldn't need to call this method. It will
    /// be called by code generated by Framec.
    pub fn transition_occurred(&mut self, transition: TransitionInstance) {
        push_to_deque(
            &self.transition_history_capacity,
            &mut self.transition_history,
            transition.clone(),
        );
        for c in &mut self.transition_callbacks {
            (**c)(&transition);
        }
    }

    /// Get the history of handled events.
    pub fn event_history(&self) -> &VecDeque<Rc<dyn MethodInstance>> {
        &self.event_history
    }

    /// Get the history of transitions that occurred.
    pub fn transition_history(&self) -> &VecDeque<TransitionInstance> {
        &self.transition_history
    }

    /// Clear the event history.
    pub fn clear_event_history(&mut self) {
        self.event_history = new_deque(&self.event_history_capacity);
    }

    /// Clear the transition history.
    pub fn clear_transition_history(&mut self) {
        self.transition_history = new_deque(&self.transition_history_capacity);
    }

    /// Set the number of events to maintain in the history. If `None`, the number of elements is
    /// unlimited.
    pub fn set_event_history_capacity(&mut self, capacity: Option<usize>) {
        resize_deque(&capacity, &mut self.event_history);
        self.event_history_capacity = capacity;
    }

    /// Set the number of transitions to maintain in the history. If `None`, the number of elements
    /// is unlimited.
    pub fn set_transition_history_capacity(&mut self, capacity: Option<usize>) {
        resize_deque(&capacity, &mut self.transition_history);
        self.transition_history_capacity = capacity;
    }

    /// Get the most recent transition. This will return `None` if either the state machine has not
    /// transitioned yet or if the capacity of the transition history is set to 0.
    pub fn last_transition(&self) -> Option<&TransitionInstance> {
        self.transition_history.back()
    }
}

impl<'a> Default for EventMonitor<'a> {
    fn default() -> Self {
        EventMonitor::new(Some(0), Some(1))
    }
}

/// Helper function to add an element to a possibly finite-sized deque.
fn push_to_deque<T>(capacity: &Option<usize>, deque: &mut VecDeque<T>, elem: T) {
    match *capacity {
        Some(cap) => {
            if cap > 0 {
                if deque.len() >= cap {
                    deque.pop_front();
                }
                deque.push_back(elem);
            }
        }
        None => deque.push_back(elem),
    };
}

/// Helper function to resize a possibly finite-sized deque.
fn resize_deque<T>(new_capacity: &Option<usize>, deque: &mut VecDeque<T>) {
    if let Some(cap) = *new_capacity {
        if deque.len() < cap {
            deque.reserve_exact(cap - deque.len());
        }
        while deque.len() > cap {
            deque.pop_front();
        }
    }
}

/// Helper function to create a possibly finite-sized deque.
fn new_deque<T>(capacity: &Option<usize>) -> VecDeque<T> {
    match *capacity {
        Some(cap) => VecDeque::with_capacity(cap),
        None => VecDeque::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{Empty, Environment};
    use crate::info::{MethodInfo, StateInfo};
    use std::any::Any;
    use std::cell::Ref;
    use std::sync::Mutex;

    mod info {
        use crate::info::*;
        use once_cell::sync::OnceCell;

        pub fn machine() -> &'static MachineInfo {
            if MACHINE_CELL.get().is_none() {
                let _ = MACHINE_CELL.set(MACHINE);
            }
            MACHINE
        }

        static MACHINE: &MachineInfo = &MachineInfo {
            name: "TestMachine",
            variables: &[],
            states: &[STATE_A, STATE_B],
            interface: &[EVENTS[0]],
            actions: ACTIONS,
            events: EVENTS,
            transitions: TRANSITIONS,
        };
        static MACHINE_CELL: OnceCell<&MachineInfo> = OnceCell::new();
        static STATE_A: &StateInfo = &StateInfo {
            machine_cell: &MACHINE_CELL,
            name: "A",
            parent: None,
            parameters: &[],
            variables: &[],
            handlers: &[EVENTS[0]],
            is_stack_pop: false,
        };
        static STATE_B: &StateInfo = &StateInfo {
            machine_cell: &MACHINE_CELL,
            name: "B",
            parent: None,
            parameters: &[],
            variables: &[],
            handlers: &[EVENTS[0]],
            is_stack_pop: false,
        };
        const ACTIONS: &[&MethodInfo] = &[];
        const EVENTS: &[&MethodInfo] = &[
            &MethodInfo {
                name: "next",
                parameters: &[],
                return_type: None,
            },
            &MethodInfo {
                name: "A:>",
                parameters: &[],
                return_type: None,
            },
            &MethodInfo {
                name: "B:>",
                parameters: &[],
                return_type: None,
            },
            &MethodInfo {
                name: "A:<",
                parameters: &[],
                return_type: None,
            },
            &MethodInfo {
                name: "B:<",
                parameters: &[],
                return_type: None,
            },
        ];
        static TRANSITIONS: &[&TransitionInfo] = &[
            &TransitionInfo {
                id: 0,
                kind: TransitionKind::Transition,
                event: EVENTS[0],
                label: "",
                source: STATE_A,
                target: STATE_B,
            },
            &TransitionInfo {
                id: 1,
                kind: TransitionKind::ChangeState,
                event: EVENTS[0],
                label: "",
                source: STATE_B,
                target: STATE_A,
            },
        ];
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
    enum TestState {
        A,
        B,
    }

    impl StateInstance for TestState {
        fn info(&self) -> &'static StateInfo {
            match self {
                TestState::A => info::machine().states[0],
                TestState::B => info::machine().states[1],
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
    enum FrameMessage {
        Enter(TestState),
        Exit(TestState),
        Next,
    }

    impl std::fmt::Display for FrameMessage {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                FrameMessage::Enter(s) => write!(f, "{:?}:>", s),
                FrameMessage::Exit(s) => write!(f, "{:?}:<", s),
                FrameMessage::Next => write!(f, "next"),
            }
        }
    }

    impl MethodInstance for FrameMessage {
        fn info(&self) -> &MethodInfo {
            info::machine().get_event(&self.to_string()).unwrap()
        }
        fn arguments(&self) -> Rc<dyn Environment> {
            Empty::new_rc()
        }
        fn return_value(&self) -> Option<Ref<dyn Any>> {
            None
        }
    }

    #[test]
    fn event_sent_callbacks() {
        let tape: Vec<String> = Vec::new();
        let tape_mutex = Mutex::new(tape);
        let mut em = EventMonitor::default();
        em.add_event_sent_callback(|e| tape_mutex.lock().unwrap().push(e.info().name.to_string()));
        em.event_sent(Rc::new(FrameMessage::Next));
        em.event_sent(Rc::new(FrameMessage::Enter(TestState::A)));
        em.event_sent(Rc::new(FrameMessage::Enter(TestState::B)));
        em.event_sent(Rc::new(FrameMessage::Next));
        em.event_sent(Rc::new(FrameMessage::Exit(TestState::A)));
        em.event_sent(Rc::new(FrameMessage::Exit(TestState::B)));
        em.event_sent(Rc::new(FrameMessage::Next));
        assert_eq!(
            *tape_mutex.lock().unwrap(),
            vec!["next", "A:>", "B:>", "next", "A:<", "B:<", "next"]
        );
    }

    #[test]
    fn event_handled_callbacks() {
        let tape: Vec<String> = Vec::new();
        let tape_mutex = Mutex::new(tape);
        let mut em = EventMonitor::default();
        em.add_event_handled_callback(|e| {
            tape_mutex.lock().unwrap().push(e.info().name.to_string())
        });
        em.event_handled(Rc::new(FrameMessage::Exit(TestState::B)));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Next));
        em.event_handled(Rc::new(FrameMessage::Exit(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::B)));
        em.event_handled(Rc::new(FrameMessage::Next));
        assert_eq!(
            *tape_mutex.lock().unwrap(),
            vec!["B:<", "A:>", "next", "A:<", "B:>", "next"]
        );
    }

    #[test]
    fn transition_callbacks() {
        let tape: Vec<String> = Vec::new();
        let tape_mutex = Mutex::new(tape);
        let mut em = EventMonitor::default();
        em.add_transition_callback(|e| {
            tape_mutex
                .lock()
                .unwrap()
                .push(format!("old: {}", e.old_state.info().name))
        });
        em.add_transition_callback(|e| {
            tape_mutex
                .lock()
                .unwrap()
                .push(format!("new: {}", e.new_state.info().name))
        });
        em.add_transition_callback(|e| {
            tape_mutex
                .lock()
                .unwrap()
                .push(format!("kind: {:?}", e.info.kind))
        });

        let a_rc = Rc::new(TestState::A);
        let b_rc = Rc::new(TestState::B);
        em.transition_occurred(TransitionInstance::change_state(
            info::machine().transitions[0],
            a_rc.clone(),
            b_rc.clone(),
        ));
        assert_eq!(
            *tape_mutex.lock().unwrap(),
            vec!["old: A", "new: B", "kind: Transition"]
        );
        tape_mutex.lock().unwrap().clear();

        em.transition_occurred(TransitionInstance::change_state(
            info::machine().transitions[1],
            b_rc,
            a_rc,
        ));
        assert_eq!(
            *tape_mutex.lock().unwrap(),
            vec!["old: B", "new: A", "kind: ChangeState"]
        );
    }

    #[test]
    fn event_history_finite() {
        let mut em = EventMonitor::new(Some(5), Some(1));
        assert!(em.event_history().is_empty());

        em.event_handled(Rc::new(FrameMessage::Next));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::B)));
        assert_eq!(
            em.event_history()
                .iter()
                .map(|e| e.info().name)
                .collect::<Vec<&str>>(),
            vec!["next", "A:>", "B:>"]
        );

        em.event_handled(Rc::new(FrameMessage::Exit(TestState::B)));
        em.event_handled(Rc::new(FrameMessage::Next));
        em.event_handled(Rc::new(FrameMessage::Exit(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Next));
        assert_eq!(
            em.event_history()
                .iter()
                .map(|e| e.info().name)
                .collect::<Vec<&str>>(),
            vec!["B:>", "B:<", "next", "A:<", "next"]
        );

        em.clear_event_history();
        assert!(em.event_history().is_empty());

        em.event_handled(Rc::new(FrameMessage::Enter(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::B)));
        assert_eq!(
            em.event_history()
                .iter()
                .map(|e| e.info().name)
                .collect::<Vec<&str>>(),
            vec!["A:>", "B:>"]
        );
    }

    #[test]
    fn event_history_infinite() {
        let mut em = EventMonitor::new(None, Some(1));
        assert!(em.event_history().is_empty());

        em.event_handled(Rc::new(FrameMessage::Next));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::B)));
        assert_eq!(
            em.event_history()
                .iter()
                .map(|e| e.info().name)
                .collect::<Vec<&str>>(),
            vec!["next", "A:>", "B:>"]
        );

        em.event_handled(Rc::new(FrameMessage::Exit(TestState::B)));
        em.event_handled(Rc::new(FrameMessage::Next));
        em.event_handled(Rc::new(FrameMessage::Exit(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Next));
        assert_eq!(
            em.event_history()
                .iter()
                .map(|e| e.info().name)
                .collect::<Vec<&str>>(),
            vec!["next", "A:>", "B:>", "B:<", "next", "A:<", "next"]
        );

        em.clear_event_history();
        assert!(em.event_history().is_empty());

        em.event_handled(Rc::new(FrameMessage::Enter(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::B)));
        assert_eq!(
            em.event_history()
                .iter()
                .map(|e| e.info().name)
                .collect::<Vec<&str>>(),
            vec!["A:>", "B:>"]
        );
    }

    #[test]
    fn event_history_disabled() {
        let mut em = EventMonitor::new(Some(0), Some(1));
        assert!(em.event_history().is_empty());

        em.event_handled(Rc::new(FrameMessage::Next));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::A)));
        em.event_handled(Rc::new(FrameMessage::Enter(TestState::B)));
        assert!(em.event_history().is_empty());

        em.clear_event_history();
        assert!(em.event_history().is_empty());
    }

    #[test]
    fn transition_history_finite() {
        let mut em = EventMonitor::new(Some(0), Some(3));
        let a = Rc::new(TestState::A);
        let b = Rc::new(TestState::B);
        let a2b =
            TransitionInstance::change_state(info::machine().transitions[0], a.clone(), b.clone());
        let b2a = TransitionInstance::change_state(info::machine().transitions[1], b, a);

        assert!(em.last_transition().is_none());
        assert!(em.transition_history().is_empty());

        em.transition_occurred(a2b.clone());
        em.transition_occurred(b2a.clone());
        assert_eq!(em.transition_history().len(), 2);

        let last = em.last_transition().unwrap();
        let first = em.transition_history().get(0).unwrap();
        assert_eq!(last.info.id, 1);
        assert_eq!(last.old_state.info().name, "B");
        assert_eq!(last.new_state.info().name, "A");
        assert_eq!(first.info.id, 0);
        assert_eq!(first.old_state.info().name, "A");
        assert_eq!(first.new_state.info().name, "B");

        em.transition_occurred(b2a.clone());
        em.transition_occurred(a2b.clone());
        assert_eq!(em.transition_history().len(), 3);
        assert_eq!(em.last_transition().unwrap().info.id, 0);
        assert_eq!(em.transition_history().get(1).unwrap().info.id, 1);
        assert_eq!(em.transition_history().get(0).unwrap().info.id, 1);

        em.clear_transition_history();
        assert!(em.transition_history().is_empty());
        em.transition_occurred(b2a);
        em.transition_occurred(a2b);
        assert_eq!(em.transition_history().len(), 2);
    }

    #[test]
    fn transition_history_infinite() {
        let mut em = EventMonitor::new(Some(0), None);
        let a = Rc::new(TestState::A);
        let b = Rc::new(TestState::B);
        let a2b =
            TransitionInstance::change_state(info::machine().transitions[0], a.clone(), b.clone());
        let b2a = TransitionInstance::change_state(info::machine().transitions[1], b, a);

        assert!(em.last_transition().is_none());
        assert!(em.transition_history().is_empty());

        em.transition_occurred(a2b.clone());
        em.transition_occurred(b2a.clone());
        assert_eq!(em.transition_history().len(), 2);

        let last = em.last_transition().unwrap();
        let first = em.transition_history().get(0).unwrap();
        assert_eq!(last.info.id, 1);
        assert_eq!(last.old_state.info().name, "B");
        assert_eq!(last.new_state.info().name, "A");
        assert_eq!(first.info.id, 0);
        assert_eq!(first.old_state.info().name, "A");
        assert_eq!(first.new_state.info().name, "B");

        em.transition_occurred(b2a.clone());
        em.transition_occurred(a2b.clone());
        assert_eq!(em.transition_history().len(), 4);
        assert_eq!(em.last_transition().unwrap().info.id, 0);
        assert_eq!(em.transition_history().get(2).unwrap().info.id, 1);
        assert_eq!(em.transition_history().get(1).unwrap().info.id, 1);
        assert_eq!(em.transition_history().get(0).unwrap().info.id, 0);

        em.clear_transition_history();
        assert!(em.transition_history().is_empty());
        em.transition_occurred(b2a);
        em.transition_occurred(a2b);
        assert_eq!(em.transition_history().len(), 2);
    }

    #[test]
    fn transition_history_disabled() {
        let mut em = EventMonitor::new(Some(0), Some(0));
        let a = Rc::new(TestState::A);
        let b = Rc::new(TestState::B);
        let a2b =
            TransitionInstance::change_state(info::machine().transitions[0], a.clone(), b.clone());
        let b2a = TransitionInstance::change_state(info::machine().transitions[1], b, a);

        assert!(em.last_transition().is_none());
        assert!(em.transition_history().is_empty());

        em.transition_occurred(a2b);
        em.transition_occurred(b2a);
        assert!(em.last_transition().is_none());
        assert!(em.transition_history().is_empty());

        em.clear_transition_history();
        assert!(em.last_transition().is_none());
        assert!(em.transition_history().is_empty());
    }
}
