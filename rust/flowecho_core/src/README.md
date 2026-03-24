# rust/flowecho_core/src

## 职责

FlowEcho Core 的生产实现目录。

## 关键文件

- `protocol.rs`: 协议与数据模型
- `error.rs`: 错误码与统一错误对象
- `crypto.rs`: 握手与 AEAD 基础能力
- `transfer.rs`: 分片传输与断点续传
- `paste_router.rs`: 粘贴路由决策
- `service.rs`: 业务服务聚合
- `ffi.rs`: 对 Flutter 暴露 FFI 接口
