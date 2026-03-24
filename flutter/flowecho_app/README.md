# flutter/flowecho_app

## 职责

FlowEcho Flutter 包，包含 Rust FFI 桥接、FlowPaste 策略与组件实现。

## 目录结构

- `lib/`: 对外 API 与内部实现
- `test/`: 单元与 widget 测试

## 关键入口

- `lib/flowecho_bridge.dart`
- `lib/src/flowecho_bridge_ffi.dart`
- `lib/src/flowpaste_panel_host.dart`
