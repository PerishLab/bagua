mod common;
use common::Fixture;
use workspace::{Edge, Failure, Geometry, Id, Operation};

#[test]
fn edges() {
    let mut fixture = Fixture::new(Geometry::new(80, 20).unwrap());
    fixture.split(fixture.creation.pane, workspace::Axis::Columns);
    for edge in [Edge::Top, Edge::Bottom] {
        fixture
            .apply(Operation::Decorate {
                session: fixture.creation.session,
                enabled: true,
                edge,
            })
            .unwrap();
        let session = fixture.world.session(fixture.creation.session).unwrap();
        let window = session.windows().next().unwrap();
        let placement = window.placements(session.canvas()).unwrap()[0];
        assert_eq!(placement.frame.geometry.rows(), 20);
        assert_eq!(placement.rect.geometry.rows(), 19);
        assert_eq!(placement.rect.y, if edge == Edge::Top { 1 } else { 0 });
        assert_eq!(session.edge(), edge);
    }
}

#[test]
fn authority() {
    let mut fixture = Fixture::new(Geometry::new(80, 20).unwrap());
    let observer = Id::new(2000).unwrap();
    fixture
        .world
        .apply(
            observer,
            Operation::Join {
                session: fixture.creation.session,
                viewport: Geometry::new(80, 20).unwrap(),
            },
        )
        .unwrap();
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.world.apply(
            observer,
            Operation::Decorate {
                session: fixture.creation.session,
                enabled: true,
                edge: Edge::Bottom
            }
        ),
        Err(Failure::Observer)
    );
    assert_eq!(fixture.world.snapshot(), before);
}

#[test]
fn atomic() {
    let mut fixture = Fixture::new(Geometry::new(80, 1).unwrap());
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Decorate {
            session: fixture.creation.session,
            enabled: true,
            edge: Edge::Bottom
        }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
}
