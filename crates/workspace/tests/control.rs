use workspace::{Failure, Geometry, Id, Session};

fn session() -> Session {
    let mut session = Session::new(Id::new(1).unwrap(), Geometry::new(213, 50).unwrap());
    session
        .join(Id::new(2).unwrap(), Geometry::new(213, 50).unwrap())
        .unwrap();
    session
        .join(Id::new(3).unwrap(), Geometry::new(213, 50).unwrap())
        .unwrap();
    session
}

#[test]
fn ownership() {
    let mut session = session();
    let first = Id::new(2).unwrap();
    let other = Id::new(3).unwrap();
    session.acquire(first).unwrap();
    assert_eq!(session.acquire(other), Err(Failure::Busy));
    assert_eq!(session.controller(), Some(first));
    assert_eq!(session.transfer(other, first), Err(Failure::Observer));
    session.transfer(first, other).unwrap();
    session.detach(first);
    assert_eq!(session.controller(), Some(other));
    session.detach(other);
    assert_eq!(session.controller(), None);
    assert_eq!(session.id(), Id::new(1).unwrap());
    assert_eq!(session.geometry(), Geometry::new(213, 50).unwrap());
}

#[test]
fn geometry() {
    let mut session = session();
    let first = Id::new(2).unwrap();
    let other = Id::new(3).unwrap();
    let narrow = Geometry::new(80, 24).unwrap();
    session.acquire(first).unwrap();
    assert_eq!(session.resize(other, narrow), Err(Failure::Observer));
    assert_eq!(session.geometry(), Geometry::new(213, 50).unwrap());
    session.fixed(first, narrow).unwrap();
    assert_eq!(
        session.resize(first, Geometry::new(160, 40).unwrap()),
        Ok(narrow)
    );
    session
        .automatic(first, Geometry::new(160, 40).unwrap())
        .unwrap();
    assert_eq!(session.geometry(), Geometry::new(160, 40).unwrap());
    assert_eq!(session.resize(first, narrow), Ok(narrow));
}
