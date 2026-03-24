use std::io::Cursor;
use std::thread;

use flowecho_core::error::ErrorCode;
use flowecho_core::transport::{
    decode_frame, encode_frame, transport_adapter, TransportEndpoint, TransportFrame,
    TransportMode,
};

#[test]
fn unsupported_transport_returns_standard_error_code() {
    let adapter = transport_adapter(TransportMode::Quic);
    let err = adapter
        .bind("127.0.0.1:0")
        .expect_err("quic placeholder must fail");
    assert_eq!(err.code, ErrorCode::UnsupportedTransport);
}

#[test]
fn frame_codec_roundtrip_preserves_type_and_payload() {
    let frame = TransportFrame::new(7, b"hello-flowecho".to_vec());
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&mut Cursor::new(encoded)).expect("decode");
    assert_eq!(decoded, frame);
}

#[test]
fn truncated_frame_payload_maps_to_invalid_request() {
    let err = decode_frame(&mut Cursor::new(vec![0, 0, 0, 5, 7, b'o']))
        .expect_err("truncated frame must fail");
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}

#[test]
fn tcp_loopback_sends_and_receives_frames_in_order() {
    let adapter = transport_adapter(TransportMode::Tcp);
    let listener = adapter.bind("127.0.0.1:0").expect("bind");
    let endpoint = listener.local_endpoint().expect("endpoint");

    let server = thread::spawn(move || {
        let mut session = listener.accept().expect("accept");
        let first = session.recv_frame().expect("first frame");
        let second = session.recv_frame().expect("second frame");
        session
            .send_frame(&TransportFrame::new(3, b"ack".to_vec()))
            .expect("send ack");
        (first, second)
    });

    let mut client = adapter.connect(&endpoint).expect("connect");
    client
        .send_frame(&TransportFrame::new(1, b"one".to_vec()))
        .expect("send first");
    client
        .send_frame(&TransportFrame::new(2, b"two".to_vec()))
        .expect("send second");

    let ack = client.recv_frame().expect("recv ack");
    let (first, second) = server.join().expect("server join");

    assert_eq!(first, TransportFrame::new(1, b"one".to_vec()));
    assert_eq!(second, TransportFrame::new(2, b"two".to_vec()));
    assert_eq!(ack, TransportFrame::new(3, b"ack".to_vec()));
}

#[test]
fn connect_to_unreachable_peer_maps_to_peer_unreachable() {
    let adapter = transport_adapter(TransportMode::Tcp);
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("probe bind");
    let endpoint = TransportEndpoint::from_socket_addr(probe.local_addr().expect("probe addr"));
    drop(probe);

    let err = adapter.connect(&endpoint).expect_err("must fail");
    assert_eq!(err.code, ErrorCode::PeerUnreachable);
}

#[test]
fn recv_after_peer_close_maps_to_session_closed() {
    let adapter = transport_adapter(TransportMode::Tcp);
    let listener = adapter.bind("127.0.0.1:0").expect("bind");
    let endpoint = listener.local_endpoint().expect("endpoint");

    let server = thread::spawn(move || {
        let session = listener.accept().expect("accept");
        drop(session);
    });

    let mut client = adapter.connect(&endpoint).expect("connect");
    server.join().expect("server join");

    let err = client.recv_frame().expect_err("must fail");
    assert_eq!(err.code, ErrorCode::SessionClosed);
}
