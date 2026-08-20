# herdr-tab-smart-rename-rs

[English](README.md)

`herdr-tab-smart-rename-rs` 是一个 Herdr 插件，可根据当前 tab 的任务上下文，将默认或数字名称自动替换为简短、有辨识度的名称。

它适合多个 coding agent 并行工作的场景：插件会读取当前 pane 和 tab 的上下文，生成贴近任务意图的名称，减少在多个任务之间切换时的认知成本。

## 功能

- 手动重命名当前 tab。
- coding agent 首次完成工作后，自动重命名默认 tab。
- 不覆盖用户手动设置的名称。
- 支持 OpenAI 及 OpenAI 兼容接口。
- 对常见终端命令生成确定性名称，尽量避免每次都调用 AI。
- Rust 单二进制实现，不依赖 Bun。

## 安装

通过 Herdr 安装：

```sh
herdr plugin install EmmetZ/herdr-tab-smart-rename-rs
```

支持的平台：

- Linux x86_64
- Linux aarch64
- macOS x86_64
- macOS Apple Silicon

## 配置 AI

通过插件 action 在 Herdr overlay 中打开私有配置文件：

```sh
herdr plugin action invoke configure-ai --plugin tab-smart-rename
```

使用 `herdr plugin install` 安装后，插件会在 Herdr 提供的私有插件配置目录创建 `provider.env`。它以 `provider.env.example` 为模板初始化，且不会覆盖已有配置。

每次模型请求前都会重新读取 `provider.env`：

```dotenv
# 默认 OpenAI 配置
OPENAI_API_KEY=你的_key
SMART_RENAME_PROVIDER=openai
SMART_RENAME_BASE_URL=https://api.openai.com/v1
SMART_RENAME_MODEL=gpt-5.6-luna
# 可选值：low、medium、high
SMART_RENAME_REASONING_EFFORT=medium
SMART_RENAME_TIMEOUT_MS=45000
```

使用其他 OpenAI 兼容服务时，设置 `SMART_RENAME_API_KEY`，并按服务商要求替换 `SMART_RENAME_PROVIDER`、`SMART_RENAME_BASE_URL` 和 `SMART_RENAME_MODEL`。若同时设置 `SMART_RENAME_API_KEY` 和 `OPENAI_API_KEY`，插件优先使用 `SMART_RENAME_API_KEY`。`SMART_RENAME_REASONING_EFFORT` 可选 `low`、`medium` 或 `high`；未配置时，非默认 provider 不会发送该字段。

## 使用

检查 AI 配置：

```sh
herdr plugin action invoke check-ai --plugin tab-smart-rename
```

立即重命名当前 tab：

```sh
herdr plugin action invoke rename-now --plugin tab-smart-rename
```

### 快捷键

在 Herdr 用户键位配置中添加以下内容，即可使用 `prefix+t` 重命名当前 tab：

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "tab-smart-rename.rename-now"
description = "smart rename current tab"
```

将 `key` 改为任意未占用的 Herdr 键位即可。该配置调用相同的手动重命名 action，可能覆盖当前 tab 已有的名称。

### 自动重命名

无需启动后台进程。插件响应 Herdr 的 `pane.agent_status_changed` 事件，且仅在同时满足下列条件时重命名 tab：

1. tab 已通过首次完成事件，该事件仅用于建立 agent 就绪基线。
2. 随后已观察到 agent 进入 `working` 状态。
3. agent 再次进入完成状态：`done`；Codex 完成一轮回复后通常为 `idle`。
4. tab 名称仍为默认名称或数字名称。

初始化阶段的完成状态不会触发重命名。已有的有意义名称也会保持不变。

## 命名规则

名称会尽量短，并优先体现当前任务意图：

- `fix-tests`
- `auth-refactor`
- `api-client`
- `docs-update`
- `ui-layout`

上下文不足时，插件不会覆盖已有的有意义名称。

## 本地构建

从源码构建并 link 插件：

```sh
cargo build --release
mkdir -p bin
install -m 0755 target/release/herdr-tab-smart-rename-rs bin/herdr-tab-smart-rename-rs
herdr plugin link .
```

## 相关文档

- [Herdr plugin API research](docs/herdr-plugin-api.md)
- [Agent status lifecycle](docs/agent-status-lifecycle.md)
- [Reference implementation notes](docs/reference-implementation.md)
- [Naming policy](docs/naming-policy.md)
- [Release packaging](docs/release-packaging.md)
