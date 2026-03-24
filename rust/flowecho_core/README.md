# rust/flowecho_core

## 职责

实现 FlowEcho Rust Core 的业务与跨语言桥接。

## 目录结构

- `src/`: 生产代码（协议/服务/路由/加密/传输/FFI）
- `tests/`: 合同与行为测试

## 关键入口

- `src/protocol.rs`
- `src/service.rs`
- `src/ffi.rs`

## 验证命令

```bash
cargo test -p flowecho_core
```
