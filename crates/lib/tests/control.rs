use protocol::{Command, Failure, Input, Intent, Outcome, Request, Status};
use workspace::{Geometry, Id, Operation, State, World};

fn request(id: u64, operation: Operation) -> Request {
    Request::identified(id, Command::Workspace(operation)).unwrap()
}

#[test]
fn shared() {
    let actor = Id::new(500).unwrap();
    let mut world = World::default();
    let response = bagua::evaluate(
        &mut world,
        actor,
        request(9, Operation::Create(Geometry::new(80, 24).unwrap())),
    );
    assert_eq!(response.request, 9);
    let Status::Applied(Outcome::Workspace(workspace::Outcome::Changed(change))) =
        response.result.unwrap()
    else {
        panic!()
    };
    assert_eq!(change.created.len(), 3);
    let creation = world.snapshot();
    let session = creation.sessions().next().unwrap().id();
    bagua::evaluate(
        &mut world,
        actor,
        request(
            10,
            Operation::Join {
                session,
                viewport: Geometry::new(80, 24).unwrap(),
            },
        ),
    )
    .result
    .unwrap();
    bagua::evaluate(
        &mut world,
        actor,
        request(
            11,
            Operation::Acquire {
                session,
                fixed: None,
            },
        ),
    )
    .result
    .unwrap();
    let before = world.snapshot();
    let mut invalid = request(12, Operation::Close(workspace::Target::Session(session)));
    invalid.version = 0;
    assert_eq!(
        bagua::evaluate(&mut world, actor, invalid).result,
        Err(Failure::Version)
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn accepted() {
    let actor = Id::new(500).unwrap();
    let observer = Id::new(600).unwrap();
    let mut world = World::default();
    let geometry = Geometry::new(80, 24).unwrap();
    let created = world.create(geometry).unwrap();
    world
        .apply(
            actor,
            Operation::Join {
                session: created.session,
                viewport: geometry,
            },
        )
        .unwrap();
    world
        .apply(
            actor,
            Operation::Acquire {
                session: created.session,
                fixed: None,
            },
        )
        .unwrap();
    let request = Request::identified(
        21,
        Command::Send {
            pane: created.pane,
            input: Input::Key(terminal::Key::Enter),
        },
    )
    .unwrap();
    assert_eq!(
        bagua::evaluate(&mut world, actor, request.clone()).result,
        Err(Failure::Workspace(workspace::Failure::State))
    );
    world.observe(created.pane, State::Running).unwrap();
    assert_eq!(
        bagua::evaluate(&mut world, observer, request.clone()).result,
        Err(Failure::Workspace(workspace::Failure::Observer))
    );
    let before = world.snapshot();
    assert!(matches!(
        bagua::evaluate(&mut world, actor, request).result,
        Ok(Status::Accepted(Intent::Input { .. }))
    ));
    assert_eq!(world.snapshot(), before);
    let oversized = Request::identified(
        22,
        Command::Send {
            pane: created.pane,
            input: Input::Text("x".repeat(terminal::CHUNK + 1)),
        },
    )
    .unwrap();
    assert_eq!(
        bagua::evaluate(&mut world, actor, oversized).result,
        Err(Failure::Payload)
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn retirement() {
    let actor = Id::new(500).unwrap();
    let geometry = Geometry::new(80, 24).unwrap();
    let mut world = World::default();
    let created = world.create(geometry).unwrap();
    world
        .apply(
            actor,
            Operation::Join {
                session: created.session,
                viewport: geometry,
            },
        )
        .unwrap();
    world
        .apply(
            actor,
            Operation::Acquire {
                session: created.session,
                fixed: None,
            },
        )
        .unwrap();
    world.observe(created.pane, State::Running).unwrap();
    let reply = bagua::evaluate(
        &mut world,
        actor,
        request(7, Operation::Close(workspace::Target::Pane(created.pane))),
    );
    let Status::Accepted(Intent::Retire(change)) = reply.result.unwrap() else {
        panic!()
    };
    assert_eq!(change.retired, vec![created.pane]);
    assert!(world.pane(created.pane).is_none());
    assert!(world.session(created.session).is_some());
    assert_eq!(
        world.observe(created.pane, State::Exited(Some(0))),
        Err(workspace::Failure::Missing)
    );
}

#[test]
fn subscription() {
    let actor = Id::new(500).unwrap();
    let geometry = Geometry::new(80, 24).unwrap();
    let mut world = World::default();
    let created = world.create(geometry).unwrap();
    let target = workspace::Target::Pane(created.pane);
    let command = Command::Subscribe {
        target,
        after: None,
    };
    assert_eq!(
        bagua::evaluate(&mut world, actor, Request::new(command.clone())).result,
        Err(Failure::Workspace(workspace::Failure::Missing))
    );
    world
        .apply(
            actor,
            Operation::Join {
                session: created.session,
                viewport: geometry,
            },
        )
        .unwrap();
    let before = world.snapshot();
    assert!(matches!(
        bagua::evaluate(&mut world, actor, Request::new(command)).result,
        Ok(Status::Accepted(Intent::Subscribe { .. }))
    ));
    assert_eq!(world.snapshot(), before);
    world
        .apply(
            actor,
            Operation::Acquire {
                session: created.session,
                fixed: None,
            },
        )
        .unwrap();
    world.apply(actor, Operation::Close(target)).unwrap();
    assert!(matches!(
        bagua::evaluate(
            &mut world,
            actor,
            Request::new(Command::Unsubscribe(target))
        )
        .result,
        Ok(Status::Accepted(Intent::Unsubscribe(_)))
    ));
}
