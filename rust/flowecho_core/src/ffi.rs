use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;

use crate::error::{ErrorCode, FlowError};
use crate::protocol::{
    ApplyPasteRequest, PairDeviceRequest, PastePolicy, PublishClipboardRequest,
    StartTransferRequest,
};
use crate::service::FlowEchoService;

fn with_request<TReq, TResp, F>(input_ptr: *const c_char, handler: F) -> *mut c_char
where
    TReq: DeserializeOwned,
    TResp: Serialize,
    F: FnOnce(TReq) -> Result<TResp, FlowError>,
{
    let response = match parse_request::<TReq>(input_ptr).and_then(handler) {
        Ok(data) => json!({ "ok": true, "data": data }),
        Err(err) => json!({
            "ok": false,
            "error": {
                "code": err.code as u16,
                "message": err.message
            }
        }),
    };

    CString::new(response.to_string())
        .expect("json without NUL")
        .into_raw()
}

fn parse_request<TReq: DeserializeOwned>(input_ptr: *const c_char) -> Result<TReq, FlowError> {
    if input_ptr.is_null() {
        return Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "null request pointer",
        ));
    }
    let input = unsafe { CStr::from_ptr(input_ptr) }
        .to_str()
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "invalid UTF-8 request"))?;
    serde_json::from_str::<TReq>(input)
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "invalid JSON request"))
}

#[no_mangle]
pub extern "C" fn flowecho_pair_device(input_ptr: *const c_char) -> *mut c_char {
    let service = FlowEchoService::default();
    with_request::<PairDeviceRequest, _, _>(input_ptr, |req| service.pair_device(req))
}

#[no_mangle]
pub extern "C" fn flowecho_publish_clipboard(input_ptr: *const c_char) -> *mut c_char {
    let service = FlowEchoService::default();
    with_request::<PublishClipboardRequest, _, _>(input_ptr, |req| service.publish_clipboard(req))
}

#[no_mangle]
pub extern "C" fn flowecho_start_transfer(input_ptr: *const c_char) -> *mut c_char {
    let service = FlowEchoService::default();
    with_request::<StartTransferRequest, _, _>(input_ptr, |req| service.start_transfer(req))
}

#[no_mangle]
pub extern "C" fn flowecho_apply_paste(input_ptr: *const c_char) -> *mut c_char {
    let service = FlowEchoService::default();
    with_request::<ApplyPasteRequest, _, _>(input_ptr, |req| service.apply_paste(req))
}

#[no_mangle]
pub extern "C" fn flowecho_set_paste_policy(input_ptr: *const c_char) -> *mut c_char {
    let service = FlowEchoService::default();
    with_request::<PastePolicy, _, _>(input_ptr, |req| service.set_paste_policy(req))
}

#[no_mangle]
pub extern "C" fn flowecho_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
