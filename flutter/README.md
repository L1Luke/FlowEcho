# flutter 模块

## 职责

提供 Flutter 侧桥接接口、策略模型与面板体验组件。

## 目录结构

- `flowecho_app/`: Flutter 包（当前 UI/桥接实现）

## 核心验证

```bash
cd flutter/flowecho_app
FLUTTER_SUPPRESS_ANALYTICS=true /opt/homebrew/bin/flutter --suppress-analytics analyze
FLUTTER_SUPPRESS_ANALYTICS=true /opt/homebrew/bin/flutter --suppress-analytics test
```
