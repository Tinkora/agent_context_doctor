# Agent Context Doctor 产品规格

## 目标

`agent_context_doctor` 用于解释哪些仓库内的指令、设置和技能文件可能影响所选路径
上的 AI 编码 Agent。它解决的常见问题是：文件位置错误、被 override 文件替代、仅
按需加载、受工作区信任限制或不属于当前宿主，导致 Agent 看起来忽略了约束。

本工具是离线、只读、隐私安全的静态解析器。输出用于排查配置发现问题，不代表运行
中宿主的完整 prompt，也不声称能够检测指令内容之间的语义冲突。

## 支持宿主与规则模型

首个版本建模以下仓库内发现规则：

| 宿主 | 指令 | 设置 | 技能 |
| --- | --- | --- | --- |
| Codex | 祖先路径上的 `AGENTS.override.md` / `AGENTS.md` | 祖先路径上的 `.codex/config.toml` 层 | 祖先路径上的 `.agents/skills/*/SKILL.md` |
| Claude Code | 祖先及嵌套目录中的 `CLAUDE.md`、`CLAUDE.local.md` 和 `.claude/CLAUDE.md` | 所选工作目录中的 `.claude/settings.json` / `settings.local.json` | 祖先及嵌套目录中的 `.claude/skills/*/SKILL.md` |
| GitHub Copilot code review | `.github/copilot-instructions.md`、`.github/instructions/**/*.instructions.md` 和根目录 `AGENTS.md` | v0.1 不建模 | v0.1 不建模 |

`--host copilot` 只表示 GitHub Copilot **code review** surface，不声称覆盖 Copilot
CLI、cloud agent 或全部 IDE。报告用 `repository-v0.1` 标识规则版本，JSON 还包含
`"copilot_surface": "code_review"`。

## 命令

```text
agent_context_doctor audit <PATH> --host codex|claude|copilot|all [--format text|json]
```

退出码：

- `0`：解析完成，未发现被忽略、受信任限制或超出范围的文件；
- `1`：解析完成，存在至少一个诊断项；
- `2`：解析不完整，或无法安全审计输入。

## 解析状态

- `active`：被静态模型选中的仓库内指令或设置；
- `shadowed`：同一目录存在 `AGENTS.override.md`，因此 Codex 不选择 `AGENTS.md`。
  `.codex/config.toml` 的 closest-wins 是键级规则，本工具不读取 TOML，因此绝不把
  整个配置文件标为被遮蔽；
- `ignored`：文件名可识别，但位置不属于宿主加载范围；
- `on-demand`：技能或路径限定指令需要任务或路径匹配，并非始终加载；
- `trust-gated`：Codex 项目配置位于所选祖先路径，但 Codex 仅在项目受信任时加载
  `.codex` 层；本工具不读取或推断用户选择；
- `out-of-scope`：文件属于另一个受支持的宿主。

每个文件都带有稳定规则标识和规范化相对路径。文本及 JSON 输出不得包含文件内容
或绝对路径。

## 隐私与安全边界

始终执行：

- 只审计明确路径及其仓库祖先路径；
- 主要使用文件系统元数据和文件名；唯一的内容访问是对 `AGENTS.override.md` 执行
  有上限的空白检测，因为 Codex 会跳过空 override 文件；
- 路径始终相对于检测到的仓库根目录；
- 遍历、元数据或资源限制失败时返回不完整结果；
- 限制祖先深度、遍历深度、总 entry 数量和文件数量；
- 遇到非 UTF-8 路径时失败关闭，并在嵌套 Git 仓库边界停止遍历。

绝不执行：

- 启动 Codex、Claude Code、Copilot、编辑器或模型；
- 读取用户主目录配置或环境变量；
- 执行 hook、插件、技能、仓库命令或配置；
- 修改、修复、移动或生成宿主配置；
- 声称检测语义冲突、有效 prompt 顺序或运行时信任状态；
- 发现文件时跟随符号链接；
- 保留、解析、报告、散列或传输指令及设置内容；
- 使用网络或发送遥测。

## 测试策略

- 单元测试覆盖宿主解析、状态序列化、相对路径脱敏、override 选择和确定性排序；
- 集成测试使用临时仓库覆盖所有宿主、错误位置、技能、路径限定指令、符号链接、
  资源限制、JSON 输出和退出码；
- CI 使用锁定依赖，在 Linux、Windows 和 macOS 上运行格式化、测试和 Clippy。

## 成功标准

- 所有宿主选择器均生成确定性的文本和 JSON 报告；
- 报告把仓库内文件分类为上述六种状态；
- 报告不包含文件内容、绝对路径、环境变量值或未脱敏的 I/O 错误；
- 除正常进程输出外不执行写操作；
- 文档不宣传运行时 prompt 检查、语义冲突检测或自动修复。

## 延后范围

- 用户级或全局配置以及运行时 prompt 检查；
- 对指令正文进行宿主特定的语义解析；
- Claude `.claude/rules/*.md`，因为区分全局规则与路径限定规则需要读取 frontmatter；
- 自动修复或配置迁移；
- 编辑器 API、模型 API、遥测与后台监控。

## 权威规则来源

- [Codex `AGENTS.md` 发现规则](https://developers.openai.com/codex/guides/agents-md)
- [Codex 项目配置优先级与信任](https://developers.openai.com/codex/config-basic)
- [Codex 仓库技能发现规则](https://developers.openai.com/codex/skills)
- [Claude Code 指令发现规则](https://code.claude.com/docs/en/memory)
- [Claude Code 设置作用域](https://code.claude.com/docs/en/settings)
- [Claude Code 技能发现规则](https://code.claude.com/docs/en/skills)
- [GitHub Copilot 自定义指令 surface 支持](https://docs.github.com/en/copilot/reference/custom-instructions-support)
