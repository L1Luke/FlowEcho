//
//  ContentView.swift
//  FlowEcho
//
//  Created by Luke on 3/24/26.
//

import SwiftUI

struct ContentView: View {
    enum FlowPasteMode: String, CaseIterable {
        case lowLatency = "低延迟"
        case lowTraffic = "低流量"
    }

    @State private var mode: FlowPasteMode = .lowLatency
    @State private var defaultSaveDirectory: String = "/Users/luke/Downloads"
    @State private var maxAutoSyncBytes: String = ""
    @State private var blockText: Bool = false
    @State private var blockImage: Bool = false
    @State private var blockFile: Bool = false
    @State private var previewMessage: String = "点击“预览决策”查看当前策略效果"
    @State private var localDeviceId: String = "ios-device"
    @State private var localAlias: String = "iPhone"
    @State private var peerIp: String = "192.168.31.20"
    @State private var peerPort: String = "47000"
    @State private var otpCode: String = ""
    @State private var sendText: String = "hello lan"
    @State private var filePath: String = "/tmp/demo.bin"
    @State private var resumeToken: String = ""
    @State private var pairingStatus: String = "未开始配对"
    @State private var transferStatus: String = "未开始传输"

    var body: some View {
        NavigationStack {
            Form {
                Section("FlowPaste 面板（MVP）") {
                    Text("iOS 仅 App 内等价入口，不支持跨 App 全局接管。")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("实网配对与实传（TCP）") {
                    Text("真机局域网配对和实传由 Flutter + Rust bridge 执行；此页只保留 App 内受控入口，不做跨 App 全局粘贴接管。")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("模式") {
                    Picker("同步策略", selection: $mode) {
                        ForEach(FlowPasteMode.allCases, id: \.self) { option in
                            Text(option.rawValue).tag(option)
                        }
                    }
                    .pickerStyle(.segmented)
                }

                Section("保存策略") {
                    TextField("默认保存目录", text: $defaultSaveDirectory)
                        .flowEchoPlainTextInput()
                    TextField("自动同步上限（字节，可空）", text: $maxAutoSyncBytes)
                        .flowEchoNumericInput()
                }

                Section("规则过滤") {
                    Toggle("屏蔽文本", isOn: $blockText)
                    Toggle("屏蔽图片", isOn: $blockImage)
                    Toggle("屏蔽文件", isOn: $blockFile)
                }

                Section("决策预览") {
                    Button("预览决策") {
                        previewMessage = buildPreview()
                    }
                    Text(previewMessage)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("配对入口") {
                    TextField("本机设备 ID", text: $localDeviceId)
                        .flowEchoPlainTextInput()
                    TextField("本机别名", text: $localAlias)
                    TextField("对端 IP", text: $peerIp)
                        .flowEchoPlainTextInput()
                    TextField("对端端口", text: $peerPort)
                        .flowEchoNumericInput()
                    TextField("6 位 OTP", text: $otpCode)
                        .flowEchoNumericInput()

                    Button("开始配对") {
                        let otp = generatedOtp()
                        otpCode = otp
                        peerPort = peerPort.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "47000" : peerPort
                        pairingStatus = "challenge_issued: ip=\(peerIp), port=\(peerPort), otp=\(otp), ttl=60s"
                    }

                    Button("确认配对") {
                        pairingStatus = buildPairingStatus()
                    }

                    Text(pairingStatus)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("传输入口") {
                    TextField("发送文本", text: $sendText, axis: .vertical)
                        .lineLimit(2...4)
                    TextField("发送文件路径", text: $filePath)
                        .flowEchoPlainTextInput()
                    TextField("恢复令牌", text: $resumeToken)
                        .flowEchoPlainTextInput()

                    Button("发送文本") {
                        transferStatus = buildTransferStatus(kind: "text")
                    }

                    Button("发送文件") {
                        if resumeToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                            resumeToken = "resume-\(Int(Date().timeIntervalSince1970))"
                        }
                        transferStatus = buildTransferStatus(kind: "file")
                    }

                    Button("恢复传输") {
                        transferStatus = buildTransferStatus(kind: "resume")
                    }

                    Text(transferStatus)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }
            .navigationTitle("FlowEcho")
        }
    }

    private func buildPreview() -> String {
        let blockedKinds = [
            blockText ? "text" : nil,
            blockImage ? "image" : nil,
            blockFile ? "file" : nil
        ]
            .compactMap { $0 }
            .joined(separator: ",")

        let maxText = maxAutoSyncBytes.trimmingCharacters(in: .whitespacesAndNewlines)
        let limit = maxText.isEmpty ? "none" : maxText

        switch mode {
        case .lowLatency:
            return "mode=low_latency, blocked=[\(blockedKinds)], max=\(limit), action=copy即同步"
        case .lowTraffic:
            return "mode=low_traffic, blocked=[\(blockedKinds)], max=\(limit), action=粘贴时请求"
        }
    }

    private func buildPairingStatus() -> String {
        let trimmedOtp = otpCode.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmedOtp.count != 6 {
            return "otp_invalid: 请输入 6 位 OTP"
        }
        return "trusted: local=\(localDeviceId), alias=\(localAlias), peer=\(peerIp):\(peerPort)"
    }

    private func buildTransferStatus(kind: String) -> String {
        switch kind {
        case "text":
            let count = sendText.count
            return "completed: text \(count)/\(count) bytes -> \(peerIp):\(peerPort)"
        case "file":
            return "pending_resume: file=\(filePath), resume=\(resumeToken), peer=\(peerIp):\(peerPort)"
        default:
            let token = resumeToken.trimmingCharacters(in: .whitespacesAndNewlines)
            if token.isEmpty {
                return "pending_resume: 请输入 resume token"
            }
            return "completed: resumed token=\(token), peer=\(peerIp):\(peerPort)"
        }
    }

    private func generatedOtp() -> String {
        let value = Int.random(in: 0...999_999)
        return String(format: "%06d", value)
    }
}

#Preview {
    ContentView()
}

private extension View {
    @ViewBuilder
    func flowEchoPlainTextInput() -> some View {
#if os(iOS) || os(visionOS)
        self
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
#else
        self
#endif
    }

    @ViewBuilder
    func flowEchoNumericInput() -> some View {
#if os(iOS) || os(visionOS)
        self.keyboardType(.numberPad)
#else
        self
#endif
    }
}
