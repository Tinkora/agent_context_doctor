# 贡献指南

[English](CONTRIBUTING.md)

扩大产品边界或修改输出契约前，请先创建 issue。保持改动聚焦，添加面向结果的测试，
并使用英文 Conventional Commits。本仓库的代码和代码注释使用英文。

提交 pull request 前请运行：

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

不得添加宿主或模型执行、超出已记录有限空白检测的内容解析、用户主目录发现、自动
修复、网络访问、遥测，也不得在报告中包含绝对路径。行为变化时应同步维护中英文文档。
