# FlowEcho

FlowEcho 是一个基于 Flutter + Rust 的跨端局域网私密快传与剪贴板协同项目，当前首发目标平台为 Windows、macOS、iOS。

## 模块层级

- `docs/`: 阶段设计与测试清单
- `rust/`: Rust Core（协议、加密、传输、路由、FFI）
- `flutter/`: Flutter 包（桥接、策略、面板与宿主接线）
- `FlowEcho/`: iOS 壳层（SwiftUI）
- `scripts/`: 自动化验证脚本

## 一键验证

```bash
bash scripts/phase-e/verify_phase_e.sh
```

## 平台边界

- 桌面端支持接管粘贴路径与原生回退组合键策略。
- iOS 仅提供 App 内等价入口，不支持跨 App 全局粘贴接管。
