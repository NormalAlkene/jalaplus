# JaLaPlus

> **声明**
>
> 本项目与 [JaLa.tools](https://jala.tools) 及其运营方没有隶属、合作或授权关系，也与日本语能力测试（JLPT）的主办机构无关。
>
> JLPT（Japanese-Language Proficiency Test，日本语能力测试）由日本国际交流基金会（The Japan Foundation）与日本国际教育支援协会（JEES）共同主办。试题内容的版权由主办机构持有（见[官方说明](https://www.jlpt.jp/e/faq/index.html)）；JLPT 的名称、标识及相关权利归其主办机构所有。本仓库不附带 JLPT 官方试题数据，使用时从网站读取和保存的内容由用户自行处理。
>
> 本项目的代码与文档均由 AI 编写，可能存在错误或遗漏。使用前请自行审查、测试并承担相应风险。
>
> 如相关权利人认为本项目侵犯其权利，请通过项目仓库的 Issue 联系，我会核实并立即删除相关内容。

JaLaPlus 是一套 JLPT 真题作答记录工具，由两部分组成：

- **JaLaPlus**：运行在 [jala.tools](https://jala.tools) 的 Tampermonkey 用户脚本。保存题目、抓取整套试卷、连续记录作答、管理收藏集并导出 JSON。
- **JaLaQuery**：Rust 命令行工具。直接读取一个或多个导出 JSON，按试卷统计正确率、估算 JLPT 分数、列出错题、比较试卷，并分析连续答题用时。

界面和命令行提示均为中文，数据保存在本机浏览器和本地文件中。

## 文档

- [用户脚本](docs/userscript.md)：网站数据、存储结构、连续答题计时、导出格式。
- [命令行工具](docs/cli.md)：安装、输入、命令、判分和 JSON 输出。
- [开发](docs/development.md)：仓库结构、测试和技术说明。

## 安装

### 用户脚本

1. 在 Firefox 中安装 [Tampermonkey](https://addons.mozilla.org/firefox/addon/tampermonkey/)。
2. 打开 Tampermonkey 管理面板，选择“新建脚本”。
3. 用 `extension/jalaplus.user.js` 的内容替换编辑器中的模板，按 `Ctrl+S` 保存。
4. 打开任意 `https://jala.tools/zh/jlpt/...` 页面，右侧下方会出现四个圆形按钮。

### 命令行工具

```bash
cd cli
cargo build --release
mkdir -p ~/.local/bin
cp target/release/jalaquery ~/.local/bin/
```

也可以在 `cli/` 目录运行 `cargo install --path .`，安装到 Cargo 的可执行文件目录。

### Skill

```bash
mkdir -p ~/.omp/agent/skills
cp -r skills/jalaquery ~/.omp/agent/skills/
```

项目级安装可放在 `.omp/skills/jalaquery/`；Claude Code 可放在 `~/.claude/skills/jalaquery/`。

## 使用

### 用户脚本

右侧浮动按钮从上到下依次是：

- **播放 / 停止**：开始或结束连续答题。开始态为绿色播放三角，答题中变为红色停止方块。
- **加号**：在题目页面保存当前题目；在试卷页面抓取整套试卷。
- **收藏集**：查看、筛选、导出和删除记录。
- **存储**：查看和管理 `localStorage`、`sessionStorage` 中的站点数据。

基本流程：

1. 打开题目，选择选项后点击加号；也可以在未作答时保存题目和正确答案。
2. 打开试卷首页，点击加号抓取整套试卷。
3. 点击绿色播放按钮开始连续答题；此后每次点击网站的“对答案”都会自动保存作答和用时。
4. 在收藏集中按试卷或题型查看记录，导出 JSON，或删除单条、所选和整批记录。

连续答题状态按标签页保存在 `sessionStorage`，刷新后恢复；再次点击红色停止按钮结束。

### JaLaQuery

```bash
# 总览
jalaquery --input export.json overview

# 一套试卷的估算分数、题型正确率和错题
jalaquery --input export.json paper n2/2020-12

# 错题和解析
jalaquery --input export.json wrong n2/2020-12 --limit 20

# 连续答题用时
jalaquery --input export.json timing

# 比较两套或多套试卷
jalaquery --input a.json --input b.json compare
```

`--input` 接受文件或目录，可重复。完整命令、判分规则和 JSON 字段见 [命令行工具](docs/cli.md)。

### Skill

安装后可在 omp 中使用 `/skill:jalaquery`；技能说明位于 `skills/jalaquery/SKILL.md`。

## 授权协议

本项目使用 [MIT License](LICENSE)。

Copyright (c) 2026 NormalAlkene
