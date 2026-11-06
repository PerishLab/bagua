use terminal::{Engine, Extent, Point};

#[test]
fn progress() {
    let mut terminal = Engine::new(Extent::new(8, 3).unwrap(), 4).unwrap();
    let mut seed = 73_u32;
    for round in 0..64 {
        let bytes = (0..512)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 24) as u8
            })
            .collect::<Vec<_>>();
        terminal.feed(&bytes).unwrap();
        let extent = Extent::new(2 + round % 7, 1 + round % 3).unwrap();
        terminal.resize(extent).unwrap();
        let snapshot = terminal.snapshot();
        assert_eq!(snapshot.extent(), extent);
        assert!(snapshot.history() <= 4);
        assert!(snapshot.cursor().position.x < extent.width());
        assert!(snapshot.cursor().position.y < extent.height());
        for x in 0..extent.width() {
            assert!(snapshot.cell(Point { x, y: 0 }).unwrap().width <= 2);
        }
    }
}
