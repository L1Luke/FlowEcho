use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flowecho_core::ffi::{
    flowecho_free_string, flowecho_latest_received_file, flowecho_latest_received_text,
    flowecho_start_pairing,
};
use flowecho_core::protocol::{PairDeviceRequest, SendFileRequest, SendTextRequest};
use flowecho_core::service::FlowEchoService;
use serde_json::Value;

#[test]
fn ffi_reports_latest_inbound_text_and_file_from_shared_service() {
    let challenge = invoke_json(
        flowecho_start_pairing,
        serde_json::json!({
            "local_device_id": "receiver-mac",
            "local_alias": "Receiver Mac",
            "peer_ip": "127.0.0.1"
        }),
    );
    let listen_port = challenge["data"]["listen_port"]
        .as_u64()
        .expect("listen_port must be present") as u16;
    let otp_code = challenge["data"]["otp_code"]
        .as_str()
        .expect("otp_code must be present")
        .to_string();

    let sender = FlowEchoService::new("127.0.0.1:0", unique_temp_dir("ffi-inbox-sender"));
    sender
        .pair_device(PairDeviceRequest {
            peer_ip: "127.0.0.1".to_string(),
            peer_port: listen_port,
            otp_code,
            local_device_id: "iphone-front".to_string(),
            local_alias: "iPhone Front".to_string(),
        })
        .expect("pair sender with shared receiver");

    sender
        .send_text(SendTextRequest {
            peer_ip: "127.0.0.1".to_string(),
            peer_port: listen_port,
            text: "ffi inbox hello".to_string(),
        })
        .expect("send text");

    let latest_text = wait_for_data(flowecho_latest_received_text);
    assert_eq!(latest_text["data"]["text"], "ffi inbox hello");
    assert_eq!(latest_text["data"]["peer_ip"], "127.0.0.1");

    let source_dir = unique_temp_dir("ffi-inbox-file");
    let source_path = source_dir.join("payload.bin");
    fs::write(&source_path, b"ffi inbox file bytes".repeat(32)).expect("write source");

    sender
        .send_file(SendFileRequest {
            peer_ip: "127.0.0.1".to_string(),
            peer_port: listen_port,
            file_path: source_path.to_string_lossy().into_owned(),
        })
        .expect("send file");

    let latest_file = wait_for_data(flowecho_latest_received_file);
    let received_path = latest_file["data"]["file_path"]
        .as_str()
        .expect("file_path");
    let received_bytes = fs::read(received_path).expect("read received");
    assert_eq!(received_bytes, b"ffi inbox file bytes".repeat(32));
    assert_eq!(latest_file["data"]["peer_ip"], "127.0.0.1");
}

fn wait_for_data(handler: unsafe extern "C" fn(*const c_char) -> *mut c_char) -> Value {
    for _ in 0..40 {
        let response = invoke_json(handler, serde_json::json!({}));
        if !response["data"].is_null() {
            return response;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("ffi inbox never produced data");
}

fn invoke_json(
    handler: unsafe extern "C" fn(*const c_char) -> *mut c_char,
    payload: Value,
) -> Value {
    let input = CString::new(payload.to_string()).expect("json input");
    let output = unsafe { handler(input.as_ptr()) };
    let body = unsafe { CStr::from_ptr(output) }
        .to_str()
        .expect("utf8 output")
        .to_string();
    flowecho_free_string(output);
    serde_json::from_str(&body).expect("valid json output")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let path = std::env::temp_dir().join(format!("flowecho-{label}-{now}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}
