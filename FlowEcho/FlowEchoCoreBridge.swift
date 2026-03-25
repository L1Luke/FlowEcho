import Foundation

nonisolated struct FlowEchoStartPairingRequest: Encodable, Sendable {
    let localDeviceId: String
    let localAlias: String
    let peerIp: String
}

nonisolated struct FlowEchoPairDeviceRequest: Encodable, Sendable {
    let peerIp: String
    let peerPort: UInt16
    let otpCode: String
    let localDeviceId: String
    let localAlias: String
}

nonisolated struct FlowEchoSendTextRequest: Encodable, Sendable {
    let peerIp: String
    let peerPort: UInt16
    let text: String
}

nonisolated struct FlowEchoSendFileRequest: Encodable, Sendable {
    let peerIp: String
    let peerPort: UInt16
    let filePath: String
}

nonisolated struct FlowEchoResumeTransferRequest: Encodable, Sendable {
    let peerIp: String
    let peerPort: UInt16
    let resumeToken: String
}

nonisolated struct FlowEchoPairingChallenge: Decodable, Sendable {
    let peerIp: String
    let listenPort: UInt16
    let otpCode: String
    let expiresAtMs: UInt64
    let attemptsRemaining: UInt8
}

nonisolated enum FlowEchoTrustState: String, Decodable, Sendable {
    case trusted
    case pending
    case revoked
}

nonisolated struct FlowEchoDeviceTrust: Decodable, Sendable {
    let deviceId: String
    let alias: String
    let trustState: FlowEchoTrustState
    let sessionKeyId: String
}

nonisolated enum FlowEchoTransferState: String, Decodable, Sendable {
    case completed
    case pendingResume = "pending_resume"
}

nonisolated struct FlowEchoTransferOutcome: Decodable, Sendable {
    let sessionId: String
    let resumeToken: String
    let state: FlowEchoTransferState
    let bytesTransferred: UInt64
    let totalBytes: UInt64
    let missingChunks: [UInt32]
    let message: String
}

nonisolated struct FlowEchoReceivedTextPayload: Decodable, Equatable, Sendable {
    let peerIp: String
    let payloadId: String
    let text: String
}

nonisolated struct FlowEchoReceivedFilePayload: Decodable, Equatable, Sendable {
    let peerIp: String
    let payloadId: String
    let payloadHash: String
    let filePath: String
}

nonisolated struct FlowEchoRpcError: Error, LocalizedError, Sendable {
    let code: Int
    let message: String

    nonisolated var errorDescription: String? {
        message
    }
}

nonisolated struct FlowEchoCoreBridge: Sendable {
    nonisolated func startPairing(_ request: FlowEchoStartPairingRequest) throws -> FlowEchoPairingChallenge {
        try invoke(flowecho_start_pairing, request: request)
    }

    nonisolated func pairDevice(_ request: FlowEchoPairDeviceRequest) throws -> FlowEchoDeviceTrust {
        try invoke(flowecho_pair_device, request: request)
    }

    nonisolated func sendText(_ request: FlowEchoSendTextRequest) throws -> FlowEchoTransferOutcome {
        try invoke(flowecho_send_text, request: request)
    }

    nonisolated func sendFile(_ request: FlowEchoSendFileRequest) throws -> FlowEchoTransferOutcome {
        try invoke(flowecho_send_file, request: request)
    }

    nonisolated func resumeTransfer(_ request: FlowEchoResumeTransferRequest) throws -> FlowEchoTransferOutcome {
        try invoke(flowecho_resume_transfer, request: request)
    }

    nonisolated func latestReceivedText() throws -> FlowEchoReceivedTextPayload? {
        try invoke(flowecho_latest_received_text, request: EmptyRequest())
    }

    nonisolated func latestReceivedFile() throws -> FlowEchoReceivedFilePayload? {
        try invoke(flowecho_latest_received_file, request: EmptyRequest())
    }

    nonisolated private func invoke<Request: Encodable, Response: Decodable>(
        _ handler: @escaping @convention(c) (UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?,
        request: Request
    ) throws -> Response {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let requestData = try encoder.encode(request)
        let requestString = String(decoding: requestData, as: UTF8.self)

        return try requestString.withCString { rawRequest in
            guard let rawResponse = handler(rawRequest) else {
                throw FlowEchoRpcError(code: -1, message: "empty rpc response")
            }
            defer { flowecho_free_string(rawResponse) }

            let responseString = String(cString: rawResponse)
            let responseData = Data(responseString.utf8)
            let envelope = try decoder.decode(RpcEnvelope<Response>.self, from: responseData)
            if envelope.ok {
                return envelope.data
            }
            let error = envelope.error ?? RpcFailure(code: -1, message: "unknown rpc error")
            throw FlowEchoRpcError(code: error.code, message: error.message)
        }
    }
}

private nonisolated struct EmptyRequest: Encodable {}

private nonisolated struct RpcEnvelope<Response: Decodable>: Decodable {
    let ok: Bool
    let data: Response
    let error: RpcFailure?
}

private nonisolated struct RpcFailure: Decodable {
    let code: Int
    let message: String
}
