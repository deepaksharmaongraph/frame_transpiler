//! This file tests features of the runtime system's event monitor.

include!(concat!(env!("OUT_DIR"), "/", "event_monitor.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use frame_runtime::*;
    use std::sync::Mutex;

    /// Test that event sent callbacks are triggered.
    #[test]
    fn event_sent() {
        let events = Mutex::new(Vec::new());
        let mut sm = EventMonitorSm::new();
        sm.event_monitor_mut().add_event_sent_callback(|e| {
            events.lock().unwrap().push(e.clone());
        });

        sm.mult(3, 5);
        sm.change();
        assert_eq!(2, events.lock().unwrap().len());
        let e1 = events.lock().unwrap()[0].clone();
        let e2 = events.lock().unwrap()[1].clone();
        assert_eq!("mult", e1.info().name);
        assert_eq!("change", e2.info().name);

        sm.reset();
        assert_eq!(3, events.lock().unwrap().len());
        let e3 = events.lock().unwrap()[2].clone();
        assert_eq!("reset", e3.info().name);
    }

    /// Test that event handled callbacks are triggered.
    #[test]
    fn event_handled() {
        let events = Mutex::new(Vec::new());
        let mut sm = EventMonitorSm::new();
        sm.event_monitor_mut().add_event_handled_callback(|e| {
            events.lock().unwrap().push(e.clone());
        });

        sm.mult(3, 5);
        sm.change();
        assert_eq!(2, events.lock().unwrap().len());
        let e1 = events.lock().unwrap()[0].clone();
        let e2 = events.lock().unwrap()[1].clone();
        assert_eq!("mult", e1.info().name);
        assert_eq!("change", e2.info().name);

        sm.reset();
        assert_eq!(3, events.lock().unwrap().len());
        let e3 = events.lock().unwrap()[2].clone();
        assert_eq!("reset", e3.info().name);
    }

    /// Test that event sent callbacks are triggered in the expected order.
    #[test]
    fn event_sent_order() {
        let events = Mutex::new(Vec::new());
        let mut sm = EventMonitorSm::new();
        sm.event_monitor_mut().add_event_sent_callback(|e| {
            events.lock().unwrap().push(e.info().name);
        });

        sm.transit(2);
        assert_eq!(EventMonitorSmState::A, sm.state);
        assert_eq!(10, events.lock().unwrap().len());
        let expected = vec![
            "transit", "A:<", "B:>", "transit", "B:<", "C:>", "transit", "C:<", "D:>", "change",
        ];
        assert_eq!(expected, *events.lock().unwrap());

        events.lock().unwrap().clear();
        sm.change();
        sm.mult(4, 6);
        sm.transit(7);
        sm.change();
        sm.reset();

        let expected = vec![
            "change", "mult", // appetizer
            "transit", "B:<", "C:>", "transit", "C:<", "D:>", "change", // main course
            "change", "reset", // dessert
        ];
        assert_eq!(expected, *events.lock().unwrap());
    }

    /// Test that event handled callbacks are triggered in the expected order.
    #[test]
    fn event_handled_order() {
        let events = Mutex::new(Vec::new());
        let mut sm = EventMonitorSm::new();
        sm.event_monitor_mut().add_event_handled_callback(|e| {
            events.lock().unwrap().push(e.info().name);
        });

        sm.transit(2);
        assert_eq!(EventMonitorSmState::A, sm.state);
        assert_eq!(10, events.lock().unwrap().len());
        let expected = vec![
            "A:<", "B:<", "C:<", "change", "D:>", "transit", "C:>", "transit", "B:>", "transit",
        ];
        assert_eq!(expected, *events.lock().unwrap());

        events.lock().unwrap().clear();
        sm.change();
        sm.mult(4, 6);
        sm.transit(7);
        sm.change();
        sm.reset();

        let expected = vec![
            "change", "mult", // appetizer
            "B:<", "C:<", "change", "D:>", "transit", "C:>", "transit", // main course
            "change", "reset", // dessert
        ];
        assert_eq!(expected, *events.lock().unwrap());
    }

    /// Test that transition callbacks are triggered in the expected order.
    #[test]
    fn transition_order() {
        let transits = Mutex::new(Vec::new());
        let mut sm = EventMonitorSm::new();
        sm.event_monitor_mut().add_transition_callback(|t| {
            transits.lock().unwrap().push(t.to_string());
        });

        sm.transit(2);
        assert_eq!(4, transits.lock().unwrap().len());
        let expected = vec!["A->B", "B->C", "C->D", "D->>A"];
        assert_eq!(expected, *transits.lock().unwrap());

        transits.lock().unwrap().clear();
        sm.change();
        sm.mult(4, 6);
        sm.transit(7);
        sm.change();
        sm.mult(7, 9);
        sm.change();
        sm.reset();
        let expected = vec!["A->>B", "B->C", "C->D", "D->>A", "A->>B", "B->>C", "C->>A"];
        assert_eq!(expected, *transits.lock().unwrap());
    }

    /// Test that event and transition callbacks are triggered in the expected relative orders.
    #[test]
    fn event_transition_order() {
        let sent = Mutex::new(Vec::new());
        let handled = Mutex::new(Vec::new());
        let mut sm = EventMonitorSm::new();
        sm.event_monitor_mut().add_event_sent_callback(|e| {
            sent.lock().unwrap().push(e.info().name.to_string());
        });
        sm.event_monitor_mut().add_event_handled_callback(|e| {
            handled.lock().unwrap().push(e.info().name.to_string());
        });
        sm.event_monitor_mut().add_transition_callback(|t| {
            sent.lock().unwrap().push(t.to_string());
            handled.lock().unwrap().push(t.to_string());
        });

        sm.transit(2);
        assert_eq!(14, sent.lock().unwrap().len());
        assert_eq!(14, handled.lock().unwrap().len());

        let sent_expected = vec![
            "transit", "A:<", "A->B", "B:>", // A->B
            "transit", "B:<", "B->C", "C:>", // B->C
            "transit", "C:<", "C->D", "D:>", // C->D
            "change", "D->>A", // D->>A
        ];
        let handled_expected = vec![
            "A:<", "A->B", "B:<", "B->C", "C:<", "C->D", "D->>A", // going down
            "change", "D:>", "transit", "C:>", "transit", "B:>", "transit", // coming back
        ];
        assert_eq!(sent_expected, *sent.lock().unwrap());
        assert_eq!(handled_expected, *handled.lock().unwrap());

        sent.lock().unwrap().clear();
        handled.lock().unwrap().clear();
        sm.change();
        sm.mult(4, 6);
        sm.transit(7);
        sm.change();
        sm.mult(7, 9);
        sm.change();
        sm.reset();
        let sent_expected = vec![
            "change", "A->>B", "mult", // change, mult
            "transit", "B:<", "B->C", "C:>", // transit
            "transit", "C:<", "C->D", "D:>", // ...
            "change", "D->>A", // ...
            "change", "A->>B", "mult", // change, mult
            "change", "B->>C", // change
            "reset", "C->>A", // reset
        ];
        let handled_expected = vec![
            "A->>B", "change", "mult", // change, mult
            "B:<", "B->C", "C:<", "C->D", "D->>A", // transit
            "change", "D:>", "transit", "C:>", "transit", // ...
            "A->>B", "change", "mult", // change, mult
            "B->>C", "change", // change
            "C->>A", "reset", // reset
        ];
        assert_eq!(sent_expected, *sent.lock().unwrap());
        assert_eq!(handled_expected, *handled.lock().unwrap());
    }

    /// Test that the event history contains the initial enter event.
    #[test]
    #[ignore]
    fn event_history_initial_enter() {
        let sm = EventMonitorSm::new();
        let history = sm.event_monitor().event_history();
        assert_eq!(1, history.len());
        assert_eq!("A:>", history.back().unwrap().info().name);
    }

    /// Test that the event history capacity works as expected.
    #[test]
    fn event_history_capacity() {
        let mut sm = EventMonitorSm::new();
        assert_eq!(Some(5), sm.event_monitor().get_event_history_capacity());
        sm.event_monitor_mut().clear_event_history();

        sm.change();
        sm.mult(3, 5);
        let history = sm.event_monitor().event_history();
        assert_eq!(2, history.len());
        assert_eq!("change", history[0].info().name);
        assert_eq!("mult", history[1].info().name);

        sm.transit(5);
        let history = sm.event_monitor().event_history();
        assert_eq!(5, history.len());
        let actual: Vec<&str> = history.iter().map(|e| e.info().name).collect();
        let expected = vec!["C:>", "transit", "C:<", "D:>", "change"];
        assert_eq!(expected, actual);

        sm.event_monitor_mut().set_event_history_capacity(Some(7));
        sm.mult(4, 6);
        sm.mult(5, 7);
        sm.change();
        let history = sm.event_monitor().event_history();
        assert_eq!(7, history.len());
        let actual: Vec<&str> = history.iter().map(|e| e.info().name).collect();
        let expected = vec!["transit", "C:<", "D:>", "change", "mult", "mult", "change"];
        assert_eq!(expected, actual);

        sm.event_monitor_mut().set_event_history_capacity(Some(3));
        let history = sm.event_monitor().event_history();
        assert_eq!(3, history.len());
        let actual: Vec<&str> = history.iter().map(|e| e.info().name).collect();
        let expected = vec!["mult", "mult", "change"];
        assert_eq!(expected, actual);

        sm.change();
        let history = sm.event_monitor().event_history();
        assert_eq!(3, history.len());
        let actual: Vec<&str> = history.iter().map(|e| e.info().name).collect();
        let expected = vec!["mult", "change", "change"];
        assert_eq!(expected, actual);

        sm.event_monitor_mut().set_event_history_capacity(None);
        sm.reset();
        sm.transit(3);
        let history = sm.event_monitor().event_history();
        assert_eq!(14, history.len());
    }

    /// Test that the transition history capacity works as expected.
    #[test]
    fn transition_history_capacity() {
        let mut sm = EventMonitorSm::new();
        assert_eq!(
            Some(3),
            sm.event_monitor().get_transition_history_capacity()
        );
        assert!(sm.event_monitor().transition_history().is_empty());

        sm.change();
        sm.mult(3, 5);
        sm.reset();
        let history = sm.event_monitor().transition_history();
        assert_eq!(2, history.len());
        assert_eq!("A->>B", history[0].to_string());
        assert_eq!("B->>A", history[1].to_string());

        sm.transit(5);
        let history = sm.event_monitor().transition_history();
        assert_eq!(3, history.len());
        let actual: Vec<String> = history.iter().map(|t| t.to_string()).collect();
        let expected = vec!["B->C", "C->D", "D->>A"];
        assert_eq!(expected, actual);

        sm.event_monitor_mut()
            .set_transition_history_capacity(Some(6));
        sm.mult(5, 7);
        sm.transit(3);
        let history = sm.event_monitor().transition_history();
        assert_eq!(6, history.len());
        let actual: Vec<String> = history.iter().map(|t| t.to_string()).collect();
        let expected = vec!["C->D", "D->>A", "A->B", "B->C", "C->D", "D->>A"];
        assert_eq!(expected, actual);

        sm.event_monitor_mut()
            .set_transition_history_capacity(Some(3));
        let history = sm.event_monitor().transition_history();
        assert_eq!(3, history.len());
        let actual: Vec<String> = history.iter().map(|t| t.to_string()).collect();
        let expected = vec!["B->C", "C->D", "D->>A"];
        assert_eq!(expected, actual);

        sm.change();
        let history = sm.event_monitor().transition_history();
        assert_eq!(3, history.len());
        let actual: Vec<String> = history.iter().map(|t| t.to_string()).collect();
        let expected = vec!["C->D", "D->>A", "A->>B"];
        assert_eq!(expected, actual);

        sm.event_monitor_mut().set_transition_history_capacity(None);
        sm.reset();
        sm.transit(4);
        sm.transit(5);
        let history = sm.event_monitor().transition_history();
        assert_eq!(12, history.len());
    }

    /// Test that return values are set in events stored in the history.
    #[test]
    fn event_history_return_value() {
        let mut sm = EventMonitorSm::new();
        sm.event_monitor_mut().clear_event_history();

        sm.change();
        sm.mult(3, 5);
        sm.change();
        sm.reset();
        let history = sm.event_monitor().event_history();
        assert!(history[0].return_value().is_some());
        assert!(history[1].return_value().is_some());
        assert!(history[2].return_value().is_some());
        assert!(history[3].return_value().is_none());
        assert_eq!(
            2,
            *history[0]
                .return_value()
                .unwrap()
                .downcast_ref::<u32>()
                .unwrap()
        );
        assert_eq!(
            15,
            *history[1]
                .return_value()
                .unwrap()
                .downcast_ref::<i32>()
                .unwrap()
        );
        assert_eq!(
            12,
            *history[2]
                .return_value()
                .unwrap()
                .downcast_ref::<u32>()
                .unwrap()
        );
    }
}
