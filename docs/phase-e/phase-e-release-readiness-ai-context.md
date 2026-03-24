# Phase E - 发布收口与 AI 上下文结构化

## 阶段目标

1. 提供统一的发布前验证入口，串联 Rust / Flutter / iOS 的关键检查。
2. 为仓库主要模块建立层级化 `README.md`，让后续 AI 与人类协作可快速获取上下文。
3. 明确平台边界与验证方法，降低跨平台交付时的信息丢失风险。

## 接口设计

### 1) 发布验证脚本

- 路径：`scripts/phase-e/verify_phase_e.sh`
- 输入：无
- 输出：标准输出日志；任何子步骤失败时立即退出非零状态
- 固定执行顺序：
  1. `cargo test -p flowecho_core`
  2. `flutter analyze`（`flutter/flowecho_app`）
  3. `flutter test`（`flutter/flowecho_app`）
  4. `xcodebuild` iOS 无签名构建检查

### 2) 模块层级 README 规范

每个模块目录下 `README.md` 至少包含：

1. 模块职责
2. 目录结构（最小树）
3. 关键入口（文件/命令）
4. 验证方式（本模块）
5. 平台边界与约束（如果存在）

## 测试清单

1. 脚本可执行性
   - `bash scripts/phase-e/verify_phase_e.sh` 必须可运行。
2. 验证链路完整性
   - Rust、Flutter、iOS 构建检查全通过。
3. README 完整性
   - docs / rust / flutter / iOS 壳层及关键子目录均有层级化 README。
4. 约束一致性
   - README 中明确 iOS 仅 App 内等价入口，不承诺跨 App 全局粘贴接管。
