use flowecho_core::error::ErrorCode;
use flowecho_core::protocol::StartTransferRequest;
use flowecho_core::service::FlowEchoService;
use flowecho_core::transfer::{create_plan, TransferAssembler};

#[test]
fn transfer_plan_has_stable_chunk_offsets_and_sizes() {
    let payload = b"0123456789abcdef";
    let plan = create_plan("payload-1", payload, 6).expect("plan");
    assert_eq!(plan.manifest.total_chunks, 3);
    assert_eq!(plan.chunks[0].offset, 0);
    assert_eq!(plan.chunks[0].size, 6);
    assert_eq!(plan.chunks[1].offset, 6);
    assert_eq!(plan.chunks[1].size, 6);
    assert_eq!(plan.chunks[2].offset, 12);
    assert_eq!(plan.chunks[2].size, 4);
}

#[test]
fn resume_offset_and_missing_chunks_are_correct() {
    let payload = b"abcdefghijklmnopqrstuvwxyz";
    let plan = create_plan("payload-2", payload, 5).expect("plan");
    let chunk0 = payload[0..5].to_vec();
    let chunk2 = payload[10..15].to_vec();

    let mut assembler = TransferAssembler::new(plan);
    assembler.apply_chunk(0, &chunk0).expect("chunk0");
    assembler.apply_chunk(2, &chunk2).expect("chunk2");

    assert_eq!(assembler.resume_offset(), 5);
    assert_eq!(assembler.next_missing_chunk(), Some(1));
    assert_eq!(assembler.missing_chunks(), vec![1, 3, 4, 5]);
}

#[test]
fn transfer_finish_reassembles_original_payload() {
    let payload = b"flowecho-phase-b-transfer";
    let plan = create_plan("payload-3", payload, 4).expect("plan");
    let chunks = plan.chunks.clone();
    let mut assembler = TransferAssembler::new(plan);

    for chunk in chunks {
        let start = chunk.offset as usize;
        let end = start + chunk.size as usize;
        assembler
            .apply_chunk(chunk.index, &payload[start..end])
            .expect("apply chunk");
    }

    let merged = assembler.finish().expect("finish");
    assert_eq!(merged, payload);
}

#[test]
fn tampered_chunk_returns_hash_mismatch() {
    let payload = b"flowecho-phase-b-security";
    let plan = create_plan("payload-4", payload, 8).expect("plan");
    let chunk = plan.chunks[1].clone();
    let mut assembler = TransferAssembler::new(plan);

    let mut tampered =
        payload[chunk.offset as usize..(chunk.offset + chunk.size as u64) as usize].to_vec();
    tampered[0] ^= 0xFF;

    let err = assembler
        .apply_chunk(chunk.index, &tampered)
        .expect_err("must fail");
    assert_eq!(err.code, ErrorCode::HashMismatch);
}

#[test]
fn service_start_transfer_returns_resume_metadata() {
    let service = FlowEchoService::default();
    let session = service
        .start_transfer(StartTransferRequest {
            payload_id: "pid-1".to_string(),
            target_device: "dev-b".to_string(),
        })
        .expect("start transfer");

    assert!(session.session_id.starts_with("tx-dev-b-pid-1-"));
    assert!(session.resume_token.starts_with("resume-"));
    assert_eq!(session.offset, 0);
    assert_eq!(session.chunk_size, 256 * 1024);
}

#[test]
fn service_start_transfer_rejects_invalid_request() {
    let service = FlowEchoService::default();
    let err = service
        .start_transfer(StartTransferRequest {
            payload_id: "".to_string(),
            target_device: " ".to_string(),
        })
        .expect_err("must fail");
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}
