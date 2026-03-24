use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flowecho_core::protocol::{PairDeviceRequest, SendFileRequest, SendTextRequest, StartPairingRequest, TransferState};
use flowecho_core::service::FlowEchoService;

static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(1);

#[test]
fn services_pair_over_tcp_and_exchange_text() {
    let receiver = FlowEchoService::new("127.0.0.1:0", unique_temp_dir("receiver"));
    let sender = FlowEchoService::new("127.0.0.1:0", unique_temp_dir("sender"));

    let challenge = receiver
        .start_pairing(StartPairingRequest {
            local_device_id: "mac-mini".to_string(),
            local_alias: "Mac mini".to_string(),
            peer_ip: "127.0.0.1".to_string(),
        })
        .expect("issue challenge");

    let trust = sender
        .pair_device(PairDeviceRequest {
            peer_ip: "127.0.0.1".to_string(),
            peer_port: challenge.listen_port,
            otp_code: challenge.otp_code.clone(),
            local_device_id: "iphone-15".to_string(),
            local_alias: "iPhone 15".to_string(),
        })
        .expect("pair device");

    assert_eq!(trust.device_id, "mac-mini");
    assert_eq!(trust.alias, "Mac mini");

    let text = "hello over real tcp";
    let outcome = sender
        .send_text(SendTextRequest {
            peer_ip: "127.0.0.1".to_string(),
            peer_port: challenge.listen_port,
            text: text.to_string(),
        })
        .expect("send text");

    assert_eq!(outcome.state, TransferState::Completed);
    assert_eq!(outcome.total_bytes, text.len() as u64);

    let received = wait_for_text(&receiver).expect("text must arrive");
    assert_eq!(received.text, text);
    assert_eq!(received.peer_ip, "127.0.0.1");
}

#[test]
fn paired_services_transfer_file_over_tcp() {
    let receiver = FlowEchoService::new("127.0.0.1:0", unique_temp_dir("receiver-file"));
    let sender = FlowEchoService::new("127.0.0.1:0", unique_temp_dir("sender-file"));

    let challenge = receiver
        .start_pairing(StartPairingRequest {
            local_device_id: "macbook-pro".to_string(),
            local_alias: "MacBook Pro".to_string(),
            peer_ip: "127.0.0.1".to_string(),
        })
        .expect("issue challenge");
    sender
        .pair_device(PairDeviceRequest {
            peer_ip: "127.0.0.1".to_string(),
            peer_port: challenge.listen_port,
            otp_code: challenge.otp_code,
            local_device_id: "iphone-pro".to_string(),
            local_alias: "iPhone Pro".to_string(),
        })
        .expect("pair device");

    let payload = b"flowecho-m4-real-file-transfer".repeat(128);
    let source_dir = unique_temp_dir("source-file");
    let source_path = source_dir.join("payload.bin");
    fs::write(&source_path, &payload).expect("write source file");

    let outcome = sender
        .send_file(SendFileRequest {
            peer_ip: "127.0.0.1".to_string(),
            peer_port: challenge.listen_port,
            file_path: source_path.to_string_lossy().into_owned(),
        })
        .expect("send file");

    assert_eq!(outcome.state, TransferState::Completed);
    assert_eq!(outcome.total_bytes, payload.len() as u64);

    let received = wait_for_file(&receiver).expect("file must arrive");
    let received_bytes = fs::read(received.file_path).expect("read received file");
    assert_eq!(received_bytes, payload);
    assert_eq!(received.peer_ip, "127.0.0.1");
}

fn wait_for_text(service: &FlowEchoService) -> Option<flowecho_core::service::ReceivedTextPayload> {
    for _ in 0..40 {
        if let Some(value) = service.latest_received_text() {
            return Some(value);
        }
        thread::sleep(Duration::from_millis(50));
    }
    None
}

fn wait_for_file(service: &FlowEchoService) -> Option<flowecho_core::service::ReceivedFilePayload> {
    for _ in 0..40 {
        if let Some(value) = service.latest_received_file() {
            return Some(value);
        }
        thread::sleep(Duration::from_millis(50));
    }
    None
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let path = std::env::temp_dir().join(format!("flowecho-{label}-{now}-{seq}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}
