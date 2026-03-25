import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @StateObject private var model = FlowEchoLanViewModel()
    @State private var isImportingFile = false

    var body: some View {
        NavigationStack {
            Form {
                Section("使用说明") {
                    Text("两台设备需在同一局域网。先在接收端输入发送端 IP 后点击“开始配对”，记下监听端口和 6 位 OTP；再在发送端填入接收端 IP、监听端口和 OTP，点击“确认配对”。")
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

                Section("对端信息") {
                    TextField("对端 IP", text: $model.peerIp)
                        .flowEchoPlainTextInput()
                    TextField("对端端口", text: $model.peerPort)
                        .flowEchoNumericInput()
                    TextField("6 位 OTP", text: $model.otpCode)
                        .flowEchoNumericInput()
                }

                Section("配对") {
                    HStack {
                        Button("开始配对") {
                            Task { await model.startPairing() }
                        }
                        .disabled(model.isWorking)

                        Button("确认配对") {
                            Task { await model.pairDevice() }
                        }
                        .disabled(model.isWorking)
                    }

                    Text(model.pairingStatus)
                        .font(.footnote)
                        .foregroundStyle(.secondary)

                    if let challenge = model.challenge {
                        Text("challenge: port=\(challenge.listenPort), otp=\(challenge.otpCode), attempts=\(challenge.attemptsRemaining)")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
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
