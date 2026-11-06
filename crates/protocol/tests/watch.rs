use protocol::{Failure, SUBSCRIPTIONS, Subscriptions};
use workspace::{Id, Target};

fn pane(value: u64) -> Target {
    Target::Pane(Id::new(value).unwrap())
}

#[test]
fn capacity() {
    let mut subscriptions = Subscriptions::default();
    for value in 1..=SUBSCRIPTIONS as u64 {
        assert!(subscriptions.subscribe(pane(value)).unwrap());
    }
    assert!(!subscriptions.subscribe(pane(1)).unwrap());
    assert_eq!(subscriptions.subscribe(pane(100)), Err(Failure::Capacity));
    assert_eq!(subscriptions.count(), SUBSCRIPTIONS);
    assert!(!subscriptions.contains(pane(100)));
    assert!(subscriptions.unsubscribe(pane(1)));
    assert!(!subscriptions.unsubscribe(pane(1)));
    assert!(subscriptions.subscribe(pane(100)).unwrap());
    assert_eq!(subscriptions.count(), SUBSCRIPTIONS);
    subscriptions.clear();
    assert!(subscriptions.empty());
}
