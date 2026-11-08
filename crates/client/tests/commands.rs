use protocol::{Command, Failure, Input, Request};
use terminal::{Extent, Key};
use workspace::{Id, Operation, Target};

#[test]
fn prefix() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let keys = client::Keys::new('q').unwrap();
    let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert_eq!(keys.translate(key, false), Ok(client::Gesture::Leader));
    assert_eq!(
        keys.translate(key, true),
        Ok(client::Gesture::Input(Input::Key(Key::Control('q'))))
    );
    let old = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    assert_eq!(
        keys.translate(old, false),
        Ok(client::Gesture::Input(Input::Key(Key::Control('g'))))
    );
    assert!(keys.translate(old, true).is_err());
    assert_eq!(
        keys.translate(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE), true),
        Ok(client::Gesture::Reload)
    );
    assert!(client::Keys::new('i').is_err());
}

#[test]
fn contract() {
    let pane = Id::new(7).unwrap();
    let view = client::View::new(Target::Pane(pane), Extent::new(80, 24).unwrap());
    let input = Input::Key(Key::Enter);
    assert_eq!(
        view.input(17, input.clone()).unwrap(),
        Request::identified(17, Command::Send { pane, input }).unwrap()
    );
    assert_eq!(
        view.focus(18).unwrap(),
        Request::identified(18, Command::Workspace(Operation::Focus(pane))).unwrap()
    );
    assert_eq!(
        view.input(0, Input::Text("text".into())),
        Err(Failure::Identity)
    );
}

#[test]
fn target() {
    let view = client::View::new(
        Target::Session(Id::new(1).unwrap()),
        Extent::new(80, 24).unwrap(),
    );
    assert_eq!(view.focus(1), Err(Failure::Target));
    assert_eq!(
        view.input(2, Input::Text("text".into())),
        Err(Failure::Target)
    );
}

#[test]
fn subscription() {
    let target = Target::Pane(Id::new(7).unwrap());
    let mut view = client::View::new(target, Extent::new(80, 24).unwrap());
    assert_eq!(
        view.subscribe(1).unwrap(),
        Request::identified(
            1,
            Command::Subscribe {
                target,
                after: None
            }
        )
        .unwrap()
    );
    let mut engine = terminal::Engine::new(Extent::new(80, 24).unwrap(), 0).unwrap();
    engine.feed(b"text").unwrap();
    view.accept(protocol::Frame::snapshot(target, engine.snapshot()))
        .unwrap();
    assert_eq!(
        view.subscribe(2).unwrap(),
        Request::identified(
            2,
            Command::Subscribe {
                target,
                after: Some(engine.revision())
            }
        )
        .unwrap()
    );
    assert_eq!(
        view.unsubscribe(3).unwrap(),
        Request::identified(3, Command::Unsubscribe(target)).unwrap()
    );
}
