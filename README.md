# herdr-tab-smart-rename-rs

`herdr-tab-smart-rename-rs` 是一个 Herdr 插件，用于根据当前 tab 中的任务上下文自动生成更有意义的 tab 名称。

它适合在多个 coding agent 并行工作时使用：当 tab 仍是默认名称或数字名称时，插件会读取当前 pane / tab 的上下文，生成一个简短、可识别的名称，降低在多个任务之间切换时的认知成本。

## 功能

- 手动重命名当前 tab。
- 在 coding agent 第一次完成后自动重命名默认 tab。
- 如果用户已经手动命名 tab，则不会覆盖用户命名。
- 支持 OpenAI 兼容接口。
- 对常见终端命令提供确定性名称，不必每次都调用 AI。
- Rust 单二进制实现，不依赖 Bun。

## 安装

通过 Herdr 安装：

```sh
herdr plugin install EmmetZ/herdr-tab-smart-rename-rs
```

支持平台：

- Linux x86_64
- Linux aarch64
- macOS x86_64
- macOS Apple Silicon

## 配置

通过插件 action 打开私有配置文件：

```sh
herdr plugin action invoke configure-ai --plugin tab-smart-rename
```

通过 `herdr plugin install` 安装时，插件会在 Herdr 提供的私有配置目录中创建
`provider.env`。该文件以 `provider.env.example` 初始化，已有配置不会被覆盖。

`provider.env` 使用以下格式（每次模型请求前重新读取）：

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

使用其他 OpenAI 兼容服务时，设置 `SMART_RENAME_API_KEY`，并按服务商要求替换 `SMART_RENAME_PROVIDER`、`SMART_RENAME_BASE_URL` 和 `SMART_RENAME_MODEL`。如果同时设置了 `SMART_RENAME_API_KEY` 和 `OPENAI_API_KEY`，插件优先使用 `SMART_RENAME_API_KEY`。`SMART_RENAME_REASONING_EFFORT` 可选值为 `low`、`medium` 或 `high`；未配置时，非默认 provider 不会发送该字段。

## 使用

检查 AI 配置：

```sh
herdr plugin action invoke check-ai --plugin tab-smart-rename
```

手动重命名当前 tab：

```sh
herdr plugin action invoke rename-now --plugin tab-smart-rename
```

### 快捷键

在 Herdr 的用户键位配置中添加以下配置，即可通过 `prefix+t` 手动重命名当前 tab：

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "tab-smart-rename.rename-now"
description = "smart rename current tab"
```

将 `key` 改为任意未占用的 Herdr 键位即可。该快捷键调用与上述 `rename-now` action 相同的手动重命名逻辑，会覆盖当前 tab 的已有名称。

自动重命名无需额外启动后台进程。插件通过 Herdr 的 `pane.agent_status_changed` 事件触发：当 coding agent 从工作状态进入完成状态后，如果当前 tab 仍是默认/数字名称，插件会自动生成名称。

## 命名规则

插件会尽量生成短名称，优先体现当前任务意图，例如：

- `fix-tests`
- `auth-refactor`
- `api-client`
- `docs-update`
- `ui-layout`

如果上下文不足，插件会保持保守，不覆盖已有的有意义名称。

## 本地构建

从源码构建并 link：

```sh
cargo build --release
mkdir -p bin
install -m 0755 target/release/herdr-tab-smart-rename-rs bin/herdr-tab-smart-rename-rs
herdr plugin link .
```

## 文档

- [Herdr plugin API research](docs/herdr-plugin-api.md)
- [Reference implementation notes](docs/reference-implementation.md)
- [Naming policy](docs/naming-policy.md)
- [Release packaging](docs/release-packaging.md)
