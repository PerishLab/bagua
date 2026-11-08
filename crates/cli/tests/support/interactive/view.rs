use super::{Command, Connection, Duration, Fixture, Message, Request};

impl Fixture {
    pub fn view(&self, pane: workspace::Id) -> protocol::wire::Screen {
        let mut connection = Connection::connect(&self.path()).unwrap();
        connection.bounded(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            connection.receive().unwrap(),
            Message::Hello {
                version: protocol::VERSION,
                ..
            }
        ));
        connection
            .send(&Request::new(Command::Workspace(
                workspace::Operation::Join {
                    session: workspace::Id::new(1).unwrap(),
                    viewport: workspace::Geometry::new(80, 24).unwrap(),
                },
            )))
            .unwrap();
        loop {
            if let Message::Finished { request: 1, result } = connection.receive().unwrap() {
                result.unwrap();
                break;
            }
        }
        connection
            .send(&Request::identified(2, Command::Inspect(pane)).unwrap())
            .unwrap();
        let mut screen = None;
        loop {
            match connection.receive().unwrap() {
                Message::Screen {
                    pane: target,
                    state,
                } => {
                    assert_eq!(target, pane);
                    screen = Some(state);
                }
                Message::Finished { request: 2, result } => {
                    result.unwrap();
                    return screen.unwrap();
                }
                _ => {}
            }
        }
    }
}
