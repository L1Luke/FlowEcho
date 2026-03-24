# FlowEcho (iOS Shell)

## 职责

iOS 原生壳层入口，承载 App 内 FlowPaste 等价入口体验。

## 关键文件

- `ContentView.swift`
- `FlowEchoApp.swift`

## 平台边界

1. iOS 仅支持 App 内入口，不支持跨 App 全局粘贴接管。
2. 全局热键接管策略由桌面端（macOS/Windows）承担。
