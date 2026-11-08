use client::{Gesture, Keys};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use protocol::Input;
use std::collections::BTreeMap;

fn event(c: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), modifiers)
}

#[test]
fn mapping() {
    let prefix = BTreeMap::from([
        ("v".into(), "pass".into()),
        ("s".into(), "split-rows".into()),
    ]);
    let direct = BTreeMap::from([
        ("Alt-q".into(), "detach".into()),
        ("Alt-1".into(), "pass".into()),
    ]);
    let keys = Keys::new('q').unwrap().bind(&prefix, &direct).unwrap();
    assert_eq!(
        keys.translate(event('q', KeyModifiers::ALT), false)
            .unwrap(),
        Gesture::Detach
    );
    assert_eq!(
        keys.translate(event('s', KeyModifiers::NONE), true)
            .unwrap(),
        Gesture::Split(workspace::Axis::Rows)
    );
    assert_eq!(
        keys.translate(event('v', KeyModifiers::NONE), true)
            .unwrap(),
        Gesture::Input(Input::Text("v".into()))
    );
    assert_eq!(
        keys.translate(event('1', KeyModifiers::ALT), false)
            .unwrap(),
        Gesture::Input(Input::Key(terminal::Key::Alt('1')))
    );
    assert_eq!(
        keys.translate(event('z', KeyModifiers::NONE), false)
            .unwrap(),
        Gesture::Input(Input::Text("z".into()))
    );
    assert_eq!(
        keys.translate(event('q', KeyModifiers::CONTROL), false)
            .unwrap(),
        Gesture::Leader
    );
    assert_eq!(
        keys.translate(event('q', KeyModifiers::CONTROL), true)
            .unwrap(),
        Gesture::Input(Input::Key(terminal::Key::Control('q')))
    );
}

#[test]
fn conflicts() {
    for values in [
        BTreeMap::from([("Ctrl-q".into(), "detach".into())]),
        BTreeMap::from([("Ctrl-Ctrl-a".into(), "detach".into())]),
        BTreeMap::from([
            ("Ctrl-Alt-a".into(), "detach".into()),
            ("Alt-Ctrl-a".into(), "pass".into()),
        ]),
        BTreeMap::from([("a".into(), "unknown".into())]),
        BTreeMap::from([("a".into(), "window-0".into())]),
        BTreeMap::from([("many".into(), "detach".into())]),
    ] {
        assert!(
            Keys::new('q')
                .unwrap()
                .bind(&values, &BTreeMap::new())
                .is_err()
        );
    }
}
