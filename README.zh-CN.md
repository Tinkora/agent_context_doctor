# Agent Context Doctor

[English](README.md)

`agent_context_doctor` 是一个离线、只读的命令行工具，用于解释 Codex、Claude
Code 和 GitHub Copilot 如何发现仓库内的指令、设置与技能文件。

当 AI Agent 看起来忽略了仓库约束时，可以用它生成隐私安全的清单，区分生效、
被遮蔽、被忽略、按需加载、受信任状态限制或不属于所选宿主范围的文件。

## 为什么需要它

不同 Agent 宿主采用不同的文件名、目录、祖先路径规则和加载条件。格式正确的文件
如果放错位置，可能完全不生效；项目设置也可能受工作区信任状态限制。本工具无需
启动宿主或向模型发送仓库数据，就能把静态发现规则变成可审查的报告。

## 安装

可下载对应平台的 Release，也可使用 Rust 1.85 或更高版本构建：

```bash
cargo install --path . --locked
```

## 使用

```bash
agent_context_doctor audit . --host all
agent_context_doctor audit crates/core --host codex --format json
```

审计仅限明确选择的 Git 仓库。报告只包含规范化的相对路径和稳定规则标识，不包含
文件内容或绝对路径。

退出码：

- `0`：解析完成，未发现诊断项；
- `1`：解析完成，存在被忽略、被遮蔽、受信任限制或超出宿主范围的文件；
- `2`：无法安全完成审计。

## 支持的仓库内文件

| 宿主 | 指令 | 设置 | 技能 |
| --- | --- | --- | --- |
| Codex | 祖先路径上的 `AGENTS.override.md` / `AGENTS.md` | 祖先路径上的 `.codex/config.toml` | 祖先路径上的 `.agents/skills/*/SKILL.md` |
| Claude Code | 祖先及嵌套目录中的 `CLAUDE.md` 变体 | 所选工作目录中的设置 | 祖先及嵌套目录中的 `.claude/skills/*/SKILL.md` |
| GitHub Copilot code review | 仓库级、路径限定及根目录 `AGENTS.md` 指令 | 暂未建模 | 暂未建模 |

`--host copilot` 明确只表示 GitHub Copilot code review surface，不代表 Copilot CLI、
cloud agent 或全部 IDE。报告使用 `repository-v0.1` 标识规则版本。宿主行为会演进；
若规则需要更新，请在 Issue 中提供权威来源。

## 非目标

本工具不会解析、保留或报告指令与设置内容。唯一的内容访问是有上限的空白检测，因为
Codex 会跳过空 `AGENTS.override.md`。它不会检查用户主目录配置、启动宿主或模型、
推断语义冲突或运行时 prompt 顺序、判断工作区是否可信、自动修复文件、访问网络或
发送遥测。
完整约束见[产品规格](docs/PRODUCT_SPEC.zh-CN.md)。

## 社区与安全

- [贡献指南](CONTRIBUTING.zh-CN.md)
- [安全策略](SECURITY.zh-CN.md)
- [支持说明](SUPPORT.zh-CN.md)
- [行为准则](CODE_OF_CONDUCT.zh-CN.md)

如果这个项目为你节省了时间，可以通过 GitHub 显示的赞助入口支持后续维护。
