use super::App;
use protocol::wire::Fault;
use workspace::Id;

impl App {
    pub fn reported(&mut self, pane: Id, fault: Fault) {
        if self.faults.contains_key(&pane) || self.faults.len() < 32 {
            self.faults.insert(pane, bounded(fault));
        }
    }

    pub(super) fn process(&mut self, pane: Id, fault: Fault) {
        if !self.session().is_some_and(|session| {
            session
                .windows()
                .any(|window| window.panes().any(|item| item.id() == pane))
        }) {
            return;
        }
        self.records.remove(&pane);
        let fault = bounded(fault);
        if self.faults.get(&pane) == Some(&fault) {
            return;
        }
        self.error(format!("Pane {}: {}", pane.value(), fault.message));
        self.reported(pane, fault);
    }
}

fn bounded(fault: Fault) -> Fault {
    Fault {
        code: fault.code.chars().take(128).collect(),
        message: fault.message.chars().take(512).collect(),
    }
}

#[cfg(test)]
mod checks {
    use super::*;
    use terminal::Extent;
    use workspace::{Entry, Geometry, Operation, Target, World};

    fn setup() -> (App, World) {
        let actor = Id::new(100).unwrap();
        let mut world = World::default();
        world
            .apply(
                actor,
                Operation::Enter(Entry {
                    name: "workspace".into(),
                    viewport: Geometry::new(80, 24).unwrap(),
                    create: true,
                    observe: false,
                    fixed: false,
                }),
            )
            .unwrap();
        let mut app = App::new(actor, Id::new(1).unwrap(), Extent::new(80, 24).unwrap());
        app.catalog(world.snapshot()).unwrap();
        (app, world)
    }

    fn fault(message: &str) -> Fault {
        Fault {
            code: "io".into(),
            message: message.into(),
        }
    }

    #[test]
    fn repeated() {
        let (mut app, _) = setup();
        let pane = Id::new(3).unwrap();
        app.process(pane, fault("missing"));
        assert_eq!(app.notice, "Pane 3: missing");
        app.status("Configuration unchanged");
        app.process(pane, fault("missing"));
        assert_eq!(app.notice, "Configuration unchanged");
        app.process(pane, fault("denied"));
        assert_eq!(app.notice, "Pane 3: denied");
    }

    #[test]
    fn scoped() {
        let (mut app, mut world) = setup();
        let pane = Id::new(3).unwrap();
        app.process(Id::new(999).unwrap(), fault("foreign"));
        assert!(app.faults.is_empty());
        app.reported(pane, fault("missing"));
        world
            .apply(app.actor(), Operation::Close(Target::Pane(pane)))
            .unwrap();
        app.catalog(world.snapshot()).unwrap();
        assert!(app.faults.is_empty());
        let (app, _) = setup();
        assert!(app.faults.is_empty());
    }

    #[test]
    fn capacity() {
        let (mut app, _) = setup();
        let pane = Id::new(3).unwrap();
        for value in 3..103 {
            app.reported(
                Id::new(value).unwrap(),
                Fault {
                    code: "c".repeat(4096),
                    message: "文".repeat(8192),
                },
            );
        }
        assert_eq!(app.faults.len(), 32);
        assert!(
            app.faults
                .values()
                .all(|fault| fault.code.len() <= 128 && fault.message.chars().count() <= 512)
        );
        app.process(pane, fault("new fault"));
        assert_eq!(app.notice, "Pane 3: new fault");
        app.status("Ready");
        app.process(pane, fault("new fault"));
        assert_eq!(app.notice, "Ready");
    }
}
