# JaLaPlus

> **声明**
>
> 本项目与 [JaLa.tools](https://jala.tools) 及其运营方没有隶属、合作或授权关系，也与日本语能力测试（JLPT）的主办机构无关。
>
> JLPT（Japanese-Language Proficiency Test，日本语能力测试）由日本国际交流基金会（The Japan Foundation）与日本国际教育支援协会（JEES）共同主办。试题内容的版权由主办机构持有（见[官方说明](https://www.jlpt.jp/e/faq/index.html)）；JLPT 的名称、标识及相关权利归其主办机构所有。本仓库不提供 JLPT 官方试题数据，使用时从网站读取和保存的内容由用户自行处理。
>
> 如相关权利人认为本项目侵犯其权利，请通过项目仓库的 Issue 联系，我会核实并立即删除相关内容。
>
> 本项目的代码与文档（包括本文档）**均使用生成式人工智能（Generative AI）技术辅助编写**，可能存在错误或遗漏。使用前请自行审查、测试并承担相应风险。

JaLaPlus 是一套对 [jala.tools](https://jala.tools) 的增强工具，主要有用户答题记录、错题记录与辅助分析等功能，旨在提升用户在该网站做题的体验。项目由两部分组成：

- **JaLaPlus**：运行在 [jala.tools](https://jala.tools) 的 Tampermonkey 用户脚本。获取并收藏题目、连续记录作答、管理收藏集并导出 JSON。
- **JaLaQuery**：Rust 命令行工具。直接读取一个或多个导出 JSON，按试卷统计正确率、估算 JLPT 分数、列出错题、比较试卷，并分析连续答题用时。

界面和命令行提示均为中文，项目产生的数据保存在本机浏览器和本地文件中，本项目不会将您的数据上传至第三方服务器中。

技术细节见 [用户脚本](docs/userscript.md)（网站数据、存储结构、计时、导出格式）、[命令行工具](docs/cli.md)（命令、判分、JSON 输出）和 [开发](docs/development.md)（结构、测试、API 约束）；关键提示词见 [提示词记录](docs/prompts.md)。

## 安装

### 用户脚本

1. 在 Firefox 中安装 [Tampermonkey](https://addons.mozilla.org/firefox/addon/tampermonkey/)。
2. 打开 Tampermonkey 管理面板，选择“新建脚本”。
3. 用 `extension/jalaplus.user.js` 的内容替换编辑器中的模板，按 `Ctrl+S` 保存。
4. 打开任意 `https://jala.tools/zh/jlpt/...` 页面，右侧下方会出现四个圆形按钮，即为安装完毕。
5. では、頑張りましょう。

### 命令行工具

```bash
cd cli
cargo build --release
mkdir -p ~/.local/bin
cp target/release/jalaquery ~/.local/bin/
```

也可以在 `cli/` 目录运行 `cargo install --path .`，安装到 Cargo 的可执行文件目录。

### Skill

仓库内的技能位于 `.agents/skills/jalaquery/`，遵循 [Agent Skills](https://agentskills.io) 约定。项目级使用时，支持该约定的 agent 会从项目根目录发现它。

用户级安装：

```bash
mkdir -p ~/.agents/skills
cp -r .agents/skills/jalaquery ~/.agents/skills/
```

若使用的 agent 只扫描自己的技能目录，把同一目录复制到其文档指定的位置即可；例如 omp 可复制到 `~/.omp/agent/skills/`。

## 使用

### 用户脚本

右侧浮动按钮从上到下依次是：

- **播放 / 停止**：开始或结束连续答题。开始态为绿色播放三角，答题中变为红色停止方块。
- **加号**：在题目页面收藏当前题目；在试卷页面抓取整套试卷并收藏所有题目。
- **收藏集**：查看、筛选、导出和删除收藏记录。
- **存储**：查看和管理网站与本项目保存在您浏览器 `localStorage`、`sessionStorage` 中的站点数据。

有两种收藏方式：

#### 连续作答并自动添加记录（推荐）

1. 打开试卷首页或具体题目。
2. 点击绿色播放按钮，启动连续答题模式。
3. 正常作答试题，此后每次点击“对答案”按钮，都会自动保存作答和用时。
4. 一套试卷作答完毕，点击红色停止按钮以结束。
5. 在收藏集中按试卷或题型查看记录，导出 JSON，或删除单条、所选和整批记录。

#### 手动添加题目

1. 打开试卷首页或具体题目。
2. （可选）在具体题目页面作答（可以不点击“对答案”按钮）。
3. 点击右侧加号按钮，将整套试题或单道题目加入收藏集。注意，jala.tools 网站保存的作答记录只有正答/错误数量，并没有历史作答选项。
4. 在收藏集中按试卷或题型查看记录，导出 JSON，或删除单条、所选和整批记录。

### JaLaQuery

推荐配合下一节中叙述的 Skill 使用，借助 Agent 辅助您分析作答记录并给出专项训练建议。

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

安装后可在 omp 或其他支持 skill 的 agent 中使用 `/skill:jalaquery`；技能说明位于 `.agents/skills/jalaquery/SKILL.md`。

## Prompts

Code is cheap, I'll show you my talk. 本项目将开发过程中的 prompts 整理出来一并开源，在 [docs/prompts.md](docs/prompts.md)。

## 授权协议

本项目使用 [MIT License](LICENSE) 授权。

