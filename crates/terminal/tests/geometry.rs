use terminal::{Extent, Revision};

#[test]
fn bounds() {
    assert_eq!(Extent::new(0, 20), None);
    assert_eq!(Extent::new(80, 0), None);
    let extent = Extent::new(106, 50).unwrap();
    assert_eq!(extent.remaining(109, 0), None);
    assert_eq!(extent.remaining(106, 0), None);
    assert_eq!(extent.remaining(0, 50), None);
    assert_eq!(extent.remaining(105, 49), Extent::new(1, 1));
}

#[test]
fn clipping() {
    let content = Extent::new(213, 50).unwrap();
    let available = Extent::new(160, 24).unwrap();
    assert_eq!(
        content.remaining(107, 0).unwrap().fit(available),
        Extent::new(106, 24).unwrap()
    );
}

#[test]
fn revision() {
    let revision = Revision::default();
    assert_eq!(revision.next(), Some(Revision::new(1)));
    assert_eq!(Revision::new(u64::MAX).next(), None);
}
