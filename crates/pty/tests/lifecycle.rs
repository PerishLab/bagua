use pty::{Event, Lifecycle, State, Transition};

#[test]
fn lifecycle() {
    let mut process = Lifecycle::default();
    assert_eq!(process.state(), State::Absent);
    assert_eq!(process.observe(Event::Started), Ok(State::Running));
    assert_eq!(
        process.observe(Event::Exited(Some(7))),
        Ok(State::Exited(Some(7)))
    );
    assert_eq!(process.observe(Event::Closed), Ok(State::Closed));
    assert_eq!(process.observe(Event::Started), Err(Transition));
}

#[test]
fn ordering() {
    let mut process = Lifecycle::default();
    assert_eq!(process.observe(Event::Exited(Some(0))), Err(Transition));
    assert_eq!(process.state(), State::Absent);
    process.observe(Event::Started).unwrap();
    assert_eq!(process.observe(Event::Closed), Err(Transition));
    assert_eq!(process.state(), State::Running);
    process.observe(Event::Exited(None)).unwrap();
    assert_eq!(process.state(), State::Exited(None));
}
