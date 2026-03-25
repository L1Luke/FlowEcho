import Foundation
import Combine
import Darwin

#if os(iOS)
import UIKit
#endif

@MainActor
final class FlowEchoLanViewModel: ObservableObject {
    @Published var localDeviceId: String
    @Published var localAlias: String
    @Published var localAddresses: [String]
    @Published var peerIp: String = ""
    @Published var peerPort: String = "45123"
    @Published var otpCode: String = ""
    @Published var sendText: String = ""
    @Published var filePath: String = ""
    @Published var resumeToken: String = ""
    @Published var pairingStatus: String = "未开始配对"
    @Published var transferStatus: String = "未开始传输"
    @Published var inboxStatus: String = "尚未收到任何内容"
    @Published var latestReceivedText: FlowEchoReceivedTextPayload?
    @Published var latestReceivedFile: FlowEchoReceivedFilePayload?
    @Published var challenge: FlowEchoPairingChallenge?
    @Published var trustedDevice: FlowEchoDeviceTrust?
    @Published var isWorking: Bool = false

    private let bridge = FlowEchoCoreBridge()
    private var pollTask: Task<Void, Never>?

    init() {
        let alias = Self.defaultAlias()
        self.localAlias = alias
        self.localDeviceId = Self.defaultDeviceId(alias: alias)
        self.localAddresses = Self.discoverLocalIPv4Addresses()
    }

    func startPolling() {
        guard pollTask == nil else {
            return
        }
        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                await self?.refreshInbox()
                try? await Task.sleep(for: .milliseconds(800))
            }
        }
    }

    func stopPolling() {
        pollTask?.cancel()
        pollTask = nil
    }

    func startPairing() async {
        await runPairingAction { [self] in
            let request = FlowEchoStartPairingRequest(
                localDeviceId: try self.normalizedLocalDeviceId(),
                localAlias: try self.normalizedLocalAlias(),
                peerIp: try self.normalizedPeerIp()
            )
            let bridge = self.bridge
            let challenge = try await self.invoke {
                try bridge.startPairing(request)
            }
            self.challenge = challenge
            self.pairingStatus = "OTP 已签发，60 秒有效，剩余尝试 \(challenge.attemptsRemaining) 次"
        }
    }

    func pairDevice() async {
        await runPairingAction { [self] in
            let peerPort = try self.normalizedPeerPort()
            let otpCode = try self.normalizedOtpCode()
            if let challenge = self.challenge,
               otpCode == challenge.otpCode,
               peerPort == challenge.listenPort {
                throw FlowEchoRpcError(
                    code: -1,
                    message: "这台设备当前是发码端。请在另一台设备上输入这里显示的 OTP 和端口，再点确认配对。"
                )
            }
            let request = FlowEchoPairDeviceRequest(
                peerIp: try self.normalizedPeerIp(),
                peerPort: peerPort,
                otpCode: otpCode,
                localDeviceId: try self.normalizedLocalDeviceId(),
                localAlias: try self.normalizedLocalAlias()
            )
            let bridge = self.bridge
            let trust = try await self.invoke {
                try bridge.pairDevice(request)
            }
            self.trustedDevice = trust
            self.pairingStatus = "设备已信任: \(trust.alias) / \(trust.deviceId)"
        }
    }

    func sendTextPacket() async {
        await runTransferAction { [self] in
            let request = FlowEchoSendTextRequest(
                peerIp: try self.normalizedPeerIp(),
                peerPort: try self.normalizedPeerPort(),
                text: try self.normalizedSendText()
            )
            let bridge = self.bridge
            let outcome = try await self.invoke {
                try bridge.sendText(request)
            }
            self.applyTransferOutcome(outcome)
        }
    }

    func sendFilePacket() async {
        await runTransferAction { [self] in
            let request = FlowEchoSendFileRequest(
                peerIp: try self.normalizedPeerIp(),
                peerPort: try self.normalizedPeerPort(),
                filePath: try self.normalizedFilePath()
            )
            let bridge = self.bridge
            let outcome = try await self.invoke {
                try bridge.sendFile(request)
            }
            self.applyTransferOutcome(outcome)
        }
    }

    func resumeTransferPacket() async {
        await runTransferAction { [self] in
            let request = FlowEchoResumeTransferRequest(
                peerIp: try self.normalizedPeerIp(),
                peerPort: try self.normalizedPeerPort(),
                resumeToken: try self.normalizedResumeToken()
            )
            let bridge = self.bridge
            let outcome = try await self.invoke {
                try bridge.resumeTransfer(request)
            }
            self.applyTransferOutcome(outcome)
        }
    }

    func refreshInbox() async {
        do {
            let bridge = self.bridge
            let latestText = try await self.invoke {
                try bridge.latestReceivedText()
            }
            if latestReceivedText != latestText {
                latestReceivedText = latestText
                if let latestText {
                    inboxStatus = "收到文本: \(latestText.payloadId)"
                }
            }

            let latestFile = try await self.invoke {
                try bridge.latestReceivedFile()
            }
            if latestReceivedFile != latestFile {
                latestReceivedFile = latestFile
                if let latestFile {
                    inboxStatus = "收到文件: \(latestFile.payloadId)"
                }
            }
        } catch {
            inboxStatus = "收件箱查询失败: \(displayMessage(for: error))"
        }
    }

    func updateFilePath(_ path: String) {
        filePath = path
    }

    private func runPairingAction(_ action: @escaping () async throws -> Void) async {
        await runAction(statusKeyPath: \.pairingStatus, action: action)
    }

    private func runTransferAction(_ action: @escaping () async throws -> Void) async {
        await runAction(statusKeyPath: \.transferStatus, action: action)
    }

    private func runAction(
        statusKeyPath: ReferenceWritableKeyPath<FlowEchoLanViewModel, String>,
        action: @escaping () async throws -> Void
    ) async {
        isWorking = true
        defer { isWorking = false }
        do {
            try await action()
        } catch {
            self[keyPath: statusKeyPath] = displayMessage(for: error)
        }
    }

    private func applyTransferOutcome(_ outcome: FlowEchoTransferOutcome) {
        resumeToken = outcome.resumeToken
        let missing = outcome.missingChunks.isEmpty
            ? "none"
            : outcome.missingChunks.map(String.init).joined(separator: ",")
        transferStatus = "\(outcome.message) | state=\(outcome.state.rawValue) | progress=\(outcome.bytesTransferred)/\(outcome.totalBytes) | missing=\(missing)"
    }

    private func normalizedLocalDeviceId() throws -> String {
        let trimmed = localDeviceId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw FlowEchoRpcError(code: -1, message: "请输入本机设备 ID")
        }
        return trimmed
    }

    private func normalizedLocalAlias() throws -> String {
        let trimmed = localAlias.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw FlowEchoRpcError(code: -1, message: "请输入本机别名")
        }
        return trimmed
    }

    private func normalizedPeerIp() throws -> String {
        let trimmed = peerIp.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw FlowEchoRpcError(code: -1, message: "请输入对端 IP")
        }
        return trimmed
    }

    private func normalizedPeerPort() throws -> UInt16 {
        guard let port = UInt16(peerPort.trimmingCharacters(in: .whitespacesAndNewlines)) else {
            throw FlowEchoRpcError(code: -1, message: "请输入有效的对端端口")
        }
        return port
    }

    private func normalizedOtpCode() throws -> String {
        let trimmed = otpCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count == 6 else {
            throw FlowEchoRpcError(code: -1, message: "请输入 6 位 OTP")
        }
        return trimmed
    }

    private func normalizedSendText() throws -> String {
        let trimmed = sendText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw FlowEchoRpcError(code: -1, message: "请输入要发送的文本")
        }
        return trimmed
    }

    private func normalizedFilePath() throws -> String {
        let trimmed = filePath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw FlowEchoRpcError(code: -1, message: "请选择要发送的文件")
        }
        guard FileManager.default.fileExists(atPath: trimmed) else {
            throw FlowEchoRpcError(code: -1, message: "发送文件不存在")
        }
        return trimmed
    }

    private func normalizedResumeToken() throws -> String {
        let trimmed = resumeToken.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw FlowEchoRpcError(code: -1, message: "请输入恢复令牌")
        }
        return trimmed
    }

    private func displayMessage(for error: Error) -> String {
        if let rpc = error as? FlowEchoRpcError {
            return rpc.message
        }
        return error.localizedDescription
    }

    private func invoke<T: Sendable>(_ operation: @escaping @Sendable () throws -> T) async throws -> T {
        try await Task.detached(priority: .userInitiated, operation: operation).value
    }

    private static func defaultAlias() -> String {
#if os(iOS)
        UIDevice.current.name
#else
        Host.current().localizedName ?? "FlowEcho Mac"
#endif
    }

    private static func defaultDeviceId(alias: String) -> String {
        let key = "flowecho.localDeviceId"
        if let stored = UserDefaults.standard.string(forKey: key), !stored.isEmpty {
            return stored
        }
        let prefix: String
#if os(iOS)
        prefix = "ios"
#else
        prefix = "mac"
#endif
        let normalizedAlias = alias
            .lowercased()
            .components(separatedBy: CharacterSet.alphanumerics.inverted)
            .filter { !$0.isEmpty }
            .joined(separator: "-")
        let suffix = UUID().uuidString.prefix(6).lowercased()
        let generated = "\(prefix)-\(normalizedAlias)-\(suffix)"
        UserDefaults.standard.set(generated, forKey: key)
        return generated
    }

    private static func discoverLocalIPv4Addresses() -> [String] {
        var results: [String] = []
        var pointer: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&pointer) == 0, let first = pointer else {
            return results
        }
        defer { freeifaddrs(pointer) }

        var cursor: UnsafeMutablePointer<ifaddrs>? = first
        while let entry = cursor?.pointee {
            defer { cursor = entry.ifa_next }

            let flags = Int32(entry.ifa_flags)
            let isUp = (flags & IFF_UP) != 0
            let isLoopback = (flags & IFF_LOOPBACK) != 0
            guard isUp, !isLoopback, let address = entry.ifa_addr else {
                continue
            }
            guard address.pointee.sa_family == UInt8(AF_INET) else {
                continue
            }

            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            let length = socklen_t(address.pointee.sa_len)
            let status = getnameinfo(
                address,
                length,
                &host,
                socklen_t(host.count),
                nil,
                0,
                NI_NUMERICHOST
            )
            guard status == 0 else {
                continue
            }

            let ip = String(cString: host)
            if !results.contains(ip) {
                results.append(ip)
            }
        }
        return results.sorted()
    }
}
