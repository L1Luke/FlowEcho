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

    var body: some View {
        NavigationStack {
            Form {
                Section("FlowPaste 面板（MVP）") {
                    Text("iOS 仅 App 内等价入口，不支持跨 App 全局接管。")
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
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    TextField("自动同步上限（字节，可空）", text: $maxAutoSyncBytes)
                        .keyboardType(.numberPad)
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
}

#Preview {
    ContentView()
}
