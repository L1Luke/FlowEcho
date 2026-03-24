use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use crate::error::{ErrorCode, FlowError, FlowResult};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TransportMode {
    Tcp,
    Quic,
    WebRtc,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TransportEndpoint {
    pub address: String,
}

impl TransportEndpoint {
    pub fn new(address: impl Into<String>) -> Self { Self { address: address.into() } }

    pub fn from_socket_addr(address: SocketAddr) -> Self { Self::new(address.to_string()) }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TransportFrame {
    pub frame_type: u8,
    pub payload: Vec<u8>,
}

impl TransportFrame {
    pub fn new(frame_type: u8, payload: Vec<u8>) -> Self { Self { frame_type, payload } }
}

pub trait TransportAdapter: Send + Sync {
    fn bind(&self, bind_addr: &str) -> FlowResult<TransportListener>;
    fn connect(&self, endpoint: &TransportEndpoint) -> FlowResult<TransportSession>;
}

pub fn transport_adapter(mode: TransportMode) -> Box<dyn TransportAdapter> {
    match mode {
        TransportMode::Tcp => Box::new(TcpTransportAdapter),
        TransportMode::Quic | TransportMode::WebRtc => {
            Box::new(UnsupportedTransportAdapter { mode })
        }
    }
}

#[derive(Debug)]
pub struct TransportListener {
    inner: TransportListenerInner,
}

#[derive(Debug)]
enum TransportListenerInner {
    Tcp(TcpListener),
}

impl TransportListener {
    pub fn accept(&self) -> FlowResult<TransportSession> {
        match &self.inner {
            TransportListenerInner::Tcp(listener) => listener
                .accept()
                .map(|(stream, _)| TransportSession {
                    inner: TransportSessionInner::Tcp(stream),
                })
                .map_err(|err| map_io_error(err, "failed to accept tcp session")),
        }
    }

    pub fn local_endpoint(&self) -> FlowResult<TransportEndpoint> {
        match &self.inner {
            TransportListenerInner::Tcp(listener) => listener
                .local_addr()
                .map(TransportEndpoint::from_socket_addr)
                .map_err(|err| map_io_error(err, "failed to resolve tcp listener address")),
        }
    }
}

#[derive(Debug)]
pub struct TransportSession {
    inner: TransportSessionInner,
}

#[derive(Debug)]
enum TransportSessionInner {
    Tcp(TcpStream),
}

impl TransportSession {
    pub fn send_frame(&mut self, frame: &TransportFrame) -> FlowResult<()> {
        let encoded = encode_frame(frame);
        match &mut self.inner {
            TransportSessionInner::Tcp(stream) => stream
                .write_all(&encoded)
                .and_then(|_| stream.flush())
                .map_err(|err| map_io_error(err, "failed to send tcp frame")),
        }
    }

    pub fn recv_frame(&mut self) -> FlowResult<TransportFrame> {
        match &mut self.inner {
            TransportSessionInner::Tcp(stream) => decode_frame(stream),
        }
    }
}

pub fn encode_frame(frame: &TransportFrame) -> Vec<u8> {
    let payload_len = frame.payload.len() + 1;
    let mut encoded = Vec::with_capacity(payload_len + 4);
    encoded.extend_from_slice(&(payload_len as u32).to_be_bytes());
    encoded.push(frame.frame_type);
    encoded.extend_from_slice(&frame.payload);
    encoded
}

pub fn decode_frame(reader: &mut impl Read) -> FlowResult<TransportFrame> {
    let mut length_bytes = [0u8; 4];
    reader
        .read_exact(&mut length_bytes)
        .map_err(|err| map_io_error(err, "failed to read frame length"))?;

    let frame_len = u32::from_be_bytes(length_bytes) as usize;
    if frame_len == 0 {
        return Err(FlowError::new(ErrorCode::InvalidRequest, "frame length must include frame type"));
    }

    let mut body = vec![0u8; frame_len];
    reader
        .read_exact(&mut body)
        .map_err(|err| match err.kind() {
            io::ErrorKind::UnexpectedEof => FlowError::new(
                ErrorCode::InvalidRequest,
                "frame payload shorter than declared length",
            ),
            _ => map_io_error(err, "failed to read frame payload"),
        })?;

    Ok(TransportFrame {
        frame_type: body[0],
        payload: body[1..].to_vec(),
    })
}

struct TcpTransportAdapter;

impl TransportAdapter for TcpTransportAdapter {
    fn bind(&self, bind_addr: &str) -> FlowResult<TransportListener> {
        TcpListener::bind(bind_addr)
            .map(|listener| TransportListener {
                inner: TransportListenerInner::Tcp(listener),
            })
            .map_err(|err| map_io_error(err, "failed to bind tcp listener"))
    }

    fn connect(&self, endpoint: &TransportEndpoint) -> FlowResult<TransportSession> {
        TcpStream::connect(&endpoint.address)
            .map(|stream| TransportSession {
                inner: TransportSessionInner::Tcp(stream),
            })
            .map_err(|err| map_io_error(err, "failed to connect tcp peer"))
    }
}

struct UnsupportedTransportAdapter {
    mode: TransportMode,
}

impl TransportAdapter for UnsupportedTransportAdapter {
    fn bind(&self, _bind_addr: &str) -> FlowResult<TransportListener> {
        Err(unsupported_transport(self.mode))
    }

    fn connect(&self, _endpoint: &TransportEndpoint) -> FlowResult<TransportSession> {
        Err(unsupported_transport(self.mode))
    }
}

fn unsupported_transport(mode: TransportMode) -> FlowError {
    FlowError::new(ErrorCode::UnsupportedTransport, format!("transport mode {} is not supported", transport_mode_name(mode)))
}

fn transport_mode_name(mode: TransportMode) -> &'static str {
    match mode {
        TransportMode::Tcp => "tcp",
        TransportMode::Quic => "quic",
        TransportMode::WebRtc => "webrtc",
    }
}

fn map_io_error(err: io::Error, context: &str) -> FlowError {
    match err.kind() {
        io::ErrorKind::ConnectionRefused
        | io::ErrorKind::ConnectionAborted
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::AddrNotAvailable
        | io::ErrorKind::TimedOut
        | io::ErrorKind::NotConnected => {
            FlowError::new(ErrorCode::PeerUnreachable, context)
        }
        io::ErrorKind::BrokenPipe | io::ErrorKind::UnexpectedEof => {
            FlowError::new(ErrorCode::SessionClosed, context)
        }
        _ => FlowError::new(ErrorCode::Internal, context),
    }
}
