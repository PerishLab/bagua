use terminal::{
    Action, Button, Direction, Engine, Extent, Failure, Input, Modifiers, Mouse, Point,
};

fn engine() -> Engine {
    Engine::new(Extent::new(300, 24).unwrap(), 0).unwrap()
}

fn event(action: Action) -> Mouse {
    Mouse {
        position: Point { x: 4, y: 6 },
        action,
        modifiers: Modifiers::default(),
    }
}

#[test]
fn sgr() {
    let mut terminal = engine();
    terminal.feed(b"\x1b[?1000h\x1b[?1006h").unwrap();
    let press = Input::Mouse(event(Action::Press(Button::Left)));
    let release = Input::Mouse(event(Action::Release(Button::Left)));
    assert_eq!(terminal.encode(press).unwrap(), b"\x1b[<0;5;7M");
    assert_eq!(terminal.encode(release).unwrap(), b"\x1b[<0;5;7m");
    let wheel = Input::Mouse(event(Action::Wheel(Direction::Up)));
    assert_eq!(terminal.encode(wheel).unwrap(), b"\x1b[<64;5;7M");
}

#[test]
fn motion() {
    let mut terminal = engine();
    let drag = Input::Mouse(event(Action::Move(Some(Button::Right))));
    let hover = Input::Mouse(event(Action::Move(None)));
    assert_eq!(terminal.encode(drag), Err(Failure::Inactive));
    terminal.feed(b"\x1b[?1002h\x1b[?1006h").unwrap();
    assert_eq!(terminal.encode(drag).unwrap(), b"\x1b[<34;5;7M");
    assert_eq!(terminal.encode(hover), Err(Failure::Inactive));
    terminal.feed(b"\x1b[?1003h").unwrap();
    assert_eq!(terminal.encode(hover).unwrap(), b"\x1b[<35;5;7M");
}

#[test]
fn legacy() {
    let mut terminal = engine();
    terminal.feed(b"\x1b[?9h").unwrap();
    let release = Input::Mouse(event(Action::Release(Button::Left)));
    assert_eq!(terminal.encode(release), Err(Failure::Inactive));
    terminal.feed(b"\x1b[?1000h").unwrap();
    assert_eq!(
        terminal.encode(release).unwrap(),
        [0x1b, b'[', b'M', 35, 37, 39]
    );
    let mut press = event(Action::Press(Button::Left));
    press.position.x = 222;
    assert_eq!(terminal.encode(Input::Mouse(press)).unwrap()[4], 255);
    press.position.x = 223;
    assert_eq!(terminal.encode(Input::Mouse(press)), Err(Failure::Position));
}

#[test]
fn bounds() {
    let mut terminal = engine();
    terminal.feed(b"\x1b[?1000h\x1b[?1006h").unwrap();
    let mut press = event(Action::Press(Button::Left));
    press.position.x = 299;
    press.modifiers = Modifiers {
        shift: true,
        alt: false,
        control: true,
    };
    assert_eq!(
        terminal.encode(Input::Mouse(press)).unwrap(),
        b"\x1b[<20;300;7M"
    );
    press.position.x = 300;
    assert_eq!(terminal.encode(Input::Mouse(press)), Err(Failure::Position));
    press.position.x = 0;
    terminal.feed(b"\x1b[?1006l\x1b[?1005h").unwrap();
    assert_eq!(
        terminal.encode(Input::Mouse(press)),
        Err(Failure::Unsupported)
    );
}
