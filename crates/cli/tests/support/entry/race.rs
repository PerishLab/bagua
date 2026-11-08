use super::{Fixture, Peer, Size};
use std::time::{Duration, Instant};

#[test]
fn startup() {
    let mut fixture = Fixture::cold();
    let mut first = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    let mut second = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    let winner = loop {
        first.pump();
        second.pump();
        if first.text().contains("bagua w1 control") && second.text().contains("RESTORED:2") {
            break true;
        }
        if second.text().contains("bagua w1 control") && first.text().contains("RESTORED:2") {
            break false;
        }
        assert!(Instant::now() < deadline, "concurrent entry did not settle");
        std::thread::sleep(Duration::from_millis(5));
    };
    let snapshot = fixture.catalog();
    assert_eq!(snapshot.sessions().count(), 1);
    let session = snapshot.sessions().next().unwrap();
    assert_eq!(session.name(), "workspace");
    assert_eq!(session.windows().count(), 1);
    assert_eq!(session.clients().count(), 1);
    assert!(fixture.shutdown());
    if winner {
        first.until("RESTORED:2");
    } else {
        second.until("RESTORED:2");
    }
}
