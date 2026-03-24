use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use flowecho_core::error::ErrorCode;
use flowecho_core::transport::{transport_adapter, TransportMode};
use flowecho_core::transfer::{
    create_file_plan, create_plan, recv_encrypted_packet, send_encrypted_packet, TransferAck,
    TransferCoordinator, TransferPacket,
};

static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(1);

#[test]
fn encrypted_text_packet_roundtrips_over_tcp() {
    let session_key = [7u8; 32];
    let adapter = transport_adapter(TransportMode::Tcp);
    let listener = adapter.bind("127.0.0.1:0").expect("bind");
    let endpoint = listener.local_endpoint().expect("endpoint");

    let server = thread::spawn(move || {
        let mut session = listener.accept().expect("accept");
        recv_encrypted_packet(&mut session, session_key).expect("recv packet")
    });

    let mut client = adapter.connect(&endpoint).expect("connect");
    send_encrypted_packet(
        &mut client,
        session_key,
        &TransferPacket::Text {
            payload_id: "text-1".to_string(),
            mime: "text/plain".to_string(),
            text: "hello from tcp".to_string(),
            hash: "ignored-by-test".to_string(),
        },
    )
    .expect("send text");

    let packet = server.join().expect("server join");
    assert_eq!(
        packet,
        TransferPacket::Text {
            payload_id: "text-1".to_string(),
            mime: "text/plain".to_string(),
            text: "hello from tcp".to_string(),
            hash: "ignored-by-test".to_string(),
        }
    );
}

#[test]
fn file_transfer_resumes_from_missing_chunk_bitmap_after_disconnect() {
    let session_key = [9u8; 32];
    let adapter = transport_adapter(TransportMode::Tcp);
    let listener = adapter.bind("127.0.0.1:0").expect("bind");
    let endpoint = listener.local_endpoint().expect("endpoint");
    let payload = b"flowecho-resume-over-real-tcp".repeat(32);
    let plan = create_plan("payload-resume", &payload, 64).expect("plan");
    let resume_token = "resume-phase-e".to_string();
    let storage_dir = unique_temp_dir("resume");

    let server = thread::spawn(move || {
        let mut coordinator = TransferCoordinator::new(storage_dir.clone()).expect("coordinator");

        let mut first_session = listener.accept().expect("accept first");
        let first_packet = recv_encrypted_packet(&mut first_session, session_key).expect("offer");
        let first_ack = match first_packet {
            TransferPacket::FileOffer { plan, resume_token } => coordinator
                .accept_offer(plan, resume_token)
                .expect("accept offer"),
            other => panic!("unexpected packet: {other:?}"),
        };
        send_encrypted_packet(
            &mut first_session,
            session_key,
            &TransferPacket::FileAck(first_ack),
        )
        .expect("send first ack");

        for _ in 0..2 {
            match recv_encrypted_packet(&mut first_session, session_key).expect("chunk") {
                TransferPacket::FileChunk {
                    resume_token,
                    index,
                    bytes,
                } => {
                    coordinator
                        .apply_chunk(&resume_token, index, &bytes)
                        .expect("apply chunk");
                }
                other => panic!("unexpected packet: {other:?}"),
            }
        }
        drop(first_session);

        let mut second_session = listener.accept().expect("accept second");
        let second_packet = recv_encrypted_packet(&mut second_session, session_key).expect("resume probe");
        let resume_ack = match second_packet {
            TransferPacket::ResumeProbe { resume_token } => coordinator
                .resume_state(&resume_token)
                .expect("resume state"),
            other => panic!("unexpected packet: {other:?}"),
        };
        send_encrypted_packet(
            &mut second_session,
            session_key,
            &TransferPacket::FileAck(resume_ack.clone()),
        )
        .expect("send resume ack");

        let resend_count = resume_ack.missing_chunks.len();
        for _ in 0..resend_count {
            match recv_encrypted_packet(&mut second_session, session_key).expect("resend chunk") {
                TransferPacket::FileChunk {
                    resume_token,
                    index,
                    bytes,
                } => {
                    coordinator
                        .apply_chunk(&resume_token, index, &bytes)
                        .expect("apply resent chunk");
                }
                other => panic!("unexpected packet: {other:?}"),
            }
        }

        match recv_encrypted_packet(&mut second_session, session_key).expect("finish") {
            TransferPacket::FileFinish { resume_token } => {
                let completed = coordinator.finish(&resume_token).expect("finish complete");
                send_encrypted_packet(
                    &mut second_session,
                    session_key,
                    &TransferPacket::FileComplete(completed.clone()),
                )
                .expect("send complete");
                completed
            }
            other => panic!("unexpected packet: {other:?}"),
        }
    });

    let mut first_client = adapter.connect(&endpoint).expect("first connect");
    send_encrypted_packet(
        &mut first_client,
        session_key,
        &TransferPacket::FileOffer {
            plan: plan.clone(),
            resume_token: resume_token.clone(),
        },
    )
    .expect("send offer");
    let _: TransferAck = match recv_encrypted_packet(&mut first_client, session_key).expect("offer ack") {
        TransferPacket::FileAck(ack) => ack,
        other => panic!("unexpected packet: {other:?}"),
    };

    for index in [0_u32, 2_u32] {
        let chunk = &plan.chunks[index as usize];
        let start = chunk.offset as usize;
        let end = start + chunk.size as usize;
        send_encrypted_packet(
            &mut first_client,
            session_key,
            &TransferPacket::FileChunk {
                resume_token: resume_token.clone(),
                index,
                bytes: payload[start..end].to_vec(),
            },
        )
        .expect("send chunk");
    }
    drop(first_client);

    let mut second_client = adapter.connect(&endpoint).expect("second connect");
    send_encrypted_packet(
        &mut second_client,
        session_key,
        &TransferPacket::ResumeProbe {
            resume_token: resume_token.clone(),
        },
    )
    .expect("send resume probe");
    let resume_ack = match recv_encrypted_packet(&mut second_client, session_key).expect("resume ack") {
        TransferPacket::FileAck(ack) => ack,
        other => panic!("unexpected packet: {other:?}"),
    };

    assert_eq!(resume_ack.missing_chunks, vec![1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);
    assert!(resume_ack.received_bitmap.iter().any(|byte| *byte != 0));

    for index in resume_ack.missing_chunks.clone() {
        let chunk = &plan.chunks[index as usize];
        let start = chunk.offset as usize;
        let end = start + chunk.size as usize;
        send_encrypted_packet(
            &mut second_client,
            session_key,
            &TransferPacket::FileChunk {
                resume_token: resume_token.clone(),
                index,
                bytes: payload[start..end].to_vec(),
            },
        )
        .expect("resend missing chunk");
    }
    send_encrypted_packet(
        &mut second_client,
        session_key,
        &TransferPacket::FileFinish {
            resume_token: resume_token.clone(),
        },
    )
    .expect("send finish");

    let completed = match recv_encrypted_packet(&mut second_client, session_key).expect("complete") {
        TransferPacket::FileComplete(completed) => completed,
        other => panic!("unexpected packet: {other:?}"),
    };
    let server_completed = server.join().expect("server join");

    assert_eq!(completed, server_completed);
    assert_eq!(completed.payload_id, "payload-resume");
    assert_eq!(completed.payload_hash, plan.manifest.hash);
    let received = fs::read(completed.file_path).expect("read completed file");
    assert_eq!(received, payload);
}

#[test]
fn tampered_chunk_reports_hash_mismatch_and_completes_after_retry() {
    let session_key = [5u8; 32];
    let adapter = transport_adapter(TransportMode::Tcp);
    let listener = adapter.bind("127.0.0.1:0").expect("bind");
    let endpoint = listener.local_endpoint().expect("endpoint");
    let payload = b"hash-mismatch-retry".repeat(48);
    let plan = create_plan("payload-retry", &payload, 48).expect("plan");
    let resume_token = "resume-retry".to_string();
    let storage_dir = unique_temp_dir("retry");

    let server = thread::spawn(move || {
        let mut coordinator = TransferCoordinator::new(storage_dir).expect("coordinator");
        let mut session = listener.accept().expect("accept");
        let offer = recv_encrypted_packet(&mut session, session_key).expect("offer");
        let offer_ack = match offer {
            TransferPacket::FileOffer { plan, resume_token } => coordinator
                .accept_offer(plan, resume_token)
                .expect("accept offer"),
            other => panic!("unexpected packet: {other:?}"),
        };
        send_encrypted_packet(&mut session, session_key, &TransferPacket::FileAck(offer_ack))
            .expect("offer ack");

        let mut mismatch_reported = false;
        loop {
            match recv_encrypted_packet(&mut session, session_key).expect("packet") {
                TransferPacket::FileChunk {
                    resume_token,
                    index,
                    bytes,
                } => {
                    let ack = match coordinator.apply_chunk(&resume_token, index, &bytes) {
                        Ok(ack) => ack,
                        Err(err) => {
                            mismatch_reported = true;
                            coordinator
                                .resume_state(&resume_token)
                                .expect("resume state")
                                .with_error(err.code)
                        }
                    };
                    send_encrypted_packet(
                        &mut session,
                        session_key,
                        &TransferPacket::FileAck(ack),
                    )
                    .expect("chunk ack");
                }
                TransferPacket::FileFinish { resume_token } => {
                    let completed = coordinator.finish(&resume_token).expect("finish");
                    send_encrypted_packet(
                        &mut session,
                        session_key,
                        &TransferPacket::FileComplete(completed.clone()),
                    )
                    .expect("complete");
                    assert!(mismatch_reported);
                    return completed;
                }
                other => panic!("unexpected packet: {other:?}"),
            }
        }
    });

    let mut client = adapter.connect(&endpoint).expect("connect");
    send_encrypted_packet(
        &mut client,
        session_key,
        &TransferPacket::FileOffer {
            plan: plan.clone(),
            resume_token: resume_token.clone(),
        },
    )
    .expect("offer");
    let _ = recv_encrypted_packet(&mut client, session_key).expect("offer ack");

    for (position, chunk) in plan.chunks.iter().enumerate() {
        let start = chunk.offset as usize;
        let end = start + chunk.size as usize;
        let mut bytes = payload[start..end].to_vec();
        if position == 1 {
            bytes[0] ^= 0xAA;
        }
        send_encrypted_packet(
            &mut client,
            session_key,
            &TransferPacket::FileChunk {
                resume_token: resume_token.clone(),
                index: chunk.index,
                bytes,
            },
        )
        .expect("send chunk");
        let ack = match recv_encrypted_packet(&mut client, session_key).expect("chunk ack") {
            TransferPacket::FileAck(ack) => ack,
            other => panic!("unexpected packet: {other:?}"),
        };
        if position == 1 {
            assert_eq!(ack.last_error_code, Some(ErrorCode::HashMismatch));
            let corrected = payload[start..end].to_vec();
            send_encrypted_packet(
                &mut client,
                session_key,
                &TransferPacket::FileChunk {
                    resume_token: resume_token.clone(),
                    index: chunk.index,
                    bytes: corrected,
                },
            )
            .expect("resend corrected chunk");
            let retry_ack = match recv_encrypted_packet(&mut client, session_key).expect("retry ack") {
                TransferPacket::FileAck(ack) => ack,
                other => panic!("unexpected packet: {other:?}"),
            };
            assert_eq!(retry_ack.last_error_code, None);
        }
    }

    send_encrypted_packet(
        &mut client,
        session_key,
        &TransferPacket::FileFinish {
            resume_token: resume_token.clone(),
        },
    )
    .expect("finish");
    let completed = match recv_encrypted_packet(&mut client, session_key).expect("complete") {
        TransferPacket::FileComplete(completed) => completed,
        other => panic!("unexpected packet: {other:?}"),
    };
    let server_completed = server.join().expect("server join");

    assert_eq!(completed, server_completed);
    assert_eq!(completed.payload_hash, plan.manifest.hash);
}

#[test]
#[ignore = "manual gate for M3 1GiB stability run"]
fn one_gib_file_transfer_stays_stable() {
    let session_key = [3u8; 32];
    let adapter = transport_adapter(TransportMode::Tcp);
    let listener = adapter.bind("127.0.0.1:0").expect("bind");
    let endpoint = listener.local_endpoint().expect("endpoint");
    let source_dir = unique_temp_dir("source-1g");
    let sink_dir = unique_temp_dir("sink-1g");
    let source_path = source_dir.join("payload-1g.bin");
    let mut source = File::create(&source_path).expect("create source");
    source
        .set_len(1024 * 1024 * 1024)
        .expect("size sparse source to 1GiB");
    source.seek(SeekFrom::Start(0)).expect("seek source");
    source.write_all(b"flowecho").expect("write prefix");
    source.seek(SeekFrom::End(-8)).expect("seek end");
    source.write_all(b"phase-e!").expect("write suffix");
    source.flush().expect("flush source");

    let plan = create_file_plan("payload-1g", &source_path, 1024 * 1024).expect("plan from file");
    let resume_token = "resume-1g".to_string();

    let server = thread::spawn(move || {
        let mut coordinator = TransferCoordinator::new(sink_dir).expect("coordinator");
        let mut session = listener.accept().expect("accept");
        let offer = recv_encrypted_packet(&mut session, session_key).expect("offer");
        let offer_ack = match offer {
            TransferPacket::FileOffer { plan, resume_token } => coordinator
                .accept_offer(plan, resume_token)
                .expect("accept offer"),
            other => panic!("unexpected packet: {other:?}"),
        };
        send_encrypted_packet(&mut session, session_key, &TransferPacket::FileAck(offer_ack))
            .expect("offer ack");

        loop {
            match recv_encrypted_packet(&mut session, session_key).expect("packet") {
                TransferPacket::FileChunk {
                    resume_token,
                    index,
                    bytes,
                } => {
                    let _ = coordinator.apply_chunk(&resume_token, index, &bytes).expect("chunk");
                }
                TransferPacket::FileFinish { resume_token } => {
                    let completed = coordinator.finish(&resume_token).expect("finish");
                    send_encrypted_packet(
                        &mut session,
                        session_key,
                        &TransferPacket::FileComplete(completed.clone()),
                    )
                    .expect("complete");
                    return completed;
                }
                other => panic!("unexpected packet: {other:?}"),
            }
        }
    });

    let mut client = adapter.connect(&endpoint).expect("connect");
    send_encrypted_packet(
        &mut client,
        session_key,
        &TransferPacket::FileOffer {
            plan: plan.clone(),
            resume_token: resume_token.clone(),
        },
    )
    .expect("offer");
    let _ = recv_encrypted_packet(&mut client, session_key).expect("offer ack");

    let mut source = File::open(&source_path).expect("open source");
    for chunk in &plan.chunks {
        let mut bytes = vec![0u8; chunk.size as usize];
        source.seek(SeekFrom::Start(chunk.offset)).expect("seek chunk");
        source.read_exact(&mut bytes).expect("read chunk");
        send_encrypted_packet(
            &mut client,
            session_key,
            &TransferPacket::FileChunk {
                resume_token: resume_token.clone(),
                index: chunk.index,
                bytes,
            },
        )
        .expect("send chunk");
    }
    send_encrypted_packet(
        &mut client,
        session_key,
        &TransferPacket::FileFinish {
            resume_token: resume_token.clone(),
        },
    )
    .expect("finish");

    let completed = match recv_encrypted_packet(&mut client, session_key).expect("complete") {
        TransferPacket::FileComplete(completed) => completed,
        other => panic!("unexpected packet: {other:?}"),
    };
    let server_completed = server.join().expect("server join");

    assert_eq!(completed, server_completed);
    assert_eq!(completed.payload_hash, plan.manifest.hash);
    assert_eq!(completed.size, 1024_u64 * 1024 * 1024);
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
