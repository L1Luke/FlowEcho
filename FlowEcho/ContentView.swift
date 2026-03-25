import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    private enum PairingRole: String, CaseIterable, Identifiable {
        case receiver
        case sender

        var id: String { rawValue }

        var title: String {
            switch self {
            case .receiver:
                return "接收端"
            case .sender:
                return "发送端"
            }
        }
    }

    @StateObject private var model = FlowEchoLanViewModel()
    @State private var isImportingFile = false
    @State private var pairingRole: PairingRole = .receiver

    var body: some View {
        NavigationStack {
            Form {
                Section("使用说明") {
                    Text("两台设备需在同一局域网。接收端只点“开始配对”生成 OTP；发送端只点“确认配对”提交对端显示的端口和 OTP。不要在同一台设备上连续点这两个按钮。")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Text("iPhone 首次连接局域网会弹出权限提示。mac 端已打开 TCP client/server sandbox 权限。")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("本机信息") {
                    TextField("本机设备 ID", text: $model.localDeviceId)
                        .flowEchoPlainTextInput()
                    TextField("本机别名", text: $model.localAlias)
                    if model.localAddresses.isEmpty {
                        Text("未检测到可用局域网 IPv4 地址")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    } else {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("本机 IPv4")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                            ForEach(model.localAddresses, id: \.self) { address in
                                Text(address)
                                    .textSelection(.enabled)
                            }
                        }
                    }
                }

                Section("配对角色") {
                    Picker("角色", selection: $pairingRole) {
                        ForEach(PairingRole.allCases) { role in
                            Text(role.title).tag(role)
                        }
                    }
                    .pickerStyle(.segmented)
                }

                Section("配对") {
                    if pairingRole == .receiver {
                        Text("接收端：填写允许来连接的发送端 IP，然后只点“开始配对”。")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                        TextField("发送端 IP", text: $model.peerIp)
                            .flowEchoPlainTextInput()

                        Button("开始配对") {
                            Task { await model.startPairing() }
                        }
                        .disabled(model.isWorking)
                    } else {
                        Text("发送端：填写接收端 IP、接收端显示的端口和 OTP，然后只点“确认配对”。")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                        TextField("接收端 IP", text: $model.peerIp)
                            .flowEchoPlainTextInput()
                        TextField("接收端端口", text: $model.peerPort)
                            .flowEchoNumericInput()
                        TextField("接收端 6 位 OTP", text: $model.otpCode)
                            .flowEchoNumericInput()

                        Button("确认配对") {
                            Task { await model.pairDevice() }
                        }
                        .disabled(model.isWorking)
                    } 

                    Text(model.pairingStatus)
                        .font(.footnote)
                        .foregroundStyle(.secondary)

                    if let challenge = model.challenge {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("本机已生成配对口令")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                            Text("监听端口: \(challenge.listenPort)")
                            Text("OTP: \(challenge.otpCode)")
                            Text("剩余尝试: \(challenge.attemptsRemaining)")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                        }
                        .textSelection(.enabled)
                    }

                    if let trust = model.trustedDevice {
                        Text("trusted: \(trust.alias) / \(trust.deviceId)")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                }

                Section("传输") {
                    TextField("发送文本", text: $model.sendText, axis: .vertical)
                        .lineLimit(2...4)
                    HStack {
                        TextField("发送文件路径", text: $model.filePath)
                            .flowEchoPlainTextInput()
                        Button("选择文件") {
                            isImportingFile = true
                        }
                    }
                    TextField("恢复令牌", text: $model.resumeToken)
                        .flowEchoPlainTextInput()

                    HStack {
                        Button("发送文本") {
                            Task { await model.sendTextPacket() }
                        }
                        .disabled(model.isWorking)

                        Button("发送文件") {
                            Task { await model.sendFilePacket() }
                        }
                        .disabled(model.isWorking)

                        Button("恢复传输") {
                            Task { await model.resumeTransferPacket() }
                        }
                        .disabled(model.isWorking)
                    }

                    Text(model.transferStatus)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }

                Section("最近接收") {
                    if let latestText = model.latestReceivedText {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("文本来自 \(latestText.peerIp)")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                            Text(latestText.text)
                        }
                    } else {
                        Text("暂无文本")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }

                    if let latestFile = model.latestReceivedFile {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("文件来自 \(latestFile.peerIp)")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                            Text(latestFile.filePath)
                                .textSelection(.enabled)
                            if FileManager.default.fileExists(atPath: latestFile.filePath) {
                                ShareLink(
                                    item: URL(fileURLWithPath: latestFile.filePath),
                                    preview: SharePreview("FlowEcho Received File")
                                ) {
                                    Text("导出最近收到的文件")
                                }
                            }
                        }
                    } else {
                        Text("暂无文件")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }

                    Text(model.inboxStatus)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }
            .navigationTitle("FlowEcho LAN Test")
        }
        .task {
            model.startPolling()
        }
        .onDisappear {
            model.stopPolling()
        }
        .fileImporter(
            isPresented: $isImportingFile,
            allowedContentTypes: [.item],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                guard let url = urls.first else {
                    return
                }
                model.updateFilePath(url.path(percentEncoded: false))
            case .failure(let error):
                model.transferStatus = error.localizedDescription
            }
        }
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
