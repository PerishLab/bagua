use protocol::{
    Command, Failure, Request,
    wire::{self, Decoder},
};

#[test]
fn fragmentation() {
    let request = Request::new(Command::create(80, 24).unwrap());
    let frame = wire::encode(&request, wire::REQUEST).unwrap();
    let mut decoder = Decoder::new(wire::REQUEST);
    for byte in &frame[..frame.len() - 1] {
        decoder.push(&[*byte]).unwrap();
        assert_eq!(decoder.take::<Request>().unwrap(), None);
    }
    decoder.push(&frame[frame.len() - 1..]).unwrap();
    assert_eq!(decoder.take::<Request>().unwrap(), Some(request.clone()));
    assert!(decoder.empty());
    let frames = frame.repeat(2);
    decoder.push(&frames).unwrap();
    assert_eq!(decoder.take::<Request>().unwrap(), Some(request.clone()));
    assert_eq!(decoder.take::<Request>().unwrap(), Some(request));
}

#[test]
fn bounds() {
    for size in [0_u32, wire::REQUEST as u32 + 1, u32::MAX] {
        let mut decoder = Decoder::new(wire::REQUEST);
        assert_eq!(decoder.push(&size.to_be_bytes()), Err(Failure::Payload));
    }
    let mut decoder = Decoder::new(1);
    assert_eq!(decoder.push(&[0; 6]), Err(Failure::Payload));
    assert_eq!(wire::encode(&"long", 1), Err(Failure::Payload));
}

#[test]
fn identities() {
    let values = [
        r#"{"version":1,"id":0,"command":"Capabilities","actor":1}"#,
        r#"{"version":1,"id":1,"command":"Unknown"}"#,
        r#"{"version":1,"id":1,"command":{"Workspace":{"Split":{"pane":1,"split":{"axis":"Columns","ratio":0}}}}}"#,
        r#"{"version":1,"id":1,"command":{"Workspace":{"Split":{"pane":1,"split":{"axis":"Columns","ratio":1000}}}}}"#,
        r#"{"version":1,"id":1,"command":{"Inspect":0}}"#,
    ];
    for value in values {
        assert!(serde_json::from_str::<Request>(value).is_err(), "{value}");
    }
    let request: Request =
        serde_json::from_str(r#"{"version":0,"id":1,"command":"Capabilities"}"#).unwrap();
    assert_eq!(request.check(), Err(Failure::Version));
}
