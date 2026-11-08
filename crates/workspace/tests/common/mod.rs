use workspace::{
    Axis, Change, Creation, Failure, Geometry, Id, Operation, Outcome, Ratio, Split, Target, World,
};

pub struct Fixture {
    pub world: World,
    pub creation: Creation,
    pub actor: Id,
}

impl Fixture {
    pub fn new(geometry: Geometry) -> Self {
        let mut world = World::default();
        let creation = world.create(geometry).unwrap();
        let actor = Id::new(1000).unwrap();
        world
            .apply(
                actor,
                Operation::Join {
                    session: creation.session,
                    viewport: geometry,
                },
            )
            .unwrap();
        world
            .apply(
                actor,
                Operation::Acquire {
                    session: creation.session,
                    fixed: None,
                },
            )
            .unwrap();
        Self {
            world,
            creation,
            actor,
        }
    }

    pub fn apply(&mut self, operation: Operation) -> Result<Change, Failure> {
        match self.world.apply(self.actor, operation)? {
            Outcome::Changed(change) | Outcome::Entered { change, .. } => Ok(change),
            Outcome::Snapshot(_) => Err(Failure::State),
        }
    }

    pub fn split(&mut self, pane: Id, axis: Axis) -> Id {
        let change = self
            .apply(Operation::Split {
                pane,
                split: Split {
                    axis,
                    ratio: Ratio::default(),
                },
            })
            .unwrap();
        let Target::Pane(id) = change.created[0] else {
            panic!("split must create a pane");
        };
        id
    }
}
