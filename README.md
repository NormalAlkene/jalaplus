# JaLaPlus

JaLaPlus 是一套自用的 JLPT 真题作答记录工具，由两部分组成：

- **JaLaPlus**：运行在 [jala.tools](https://jala.tools) 的 Tampermonkey 用户脚本。用于保存题目、导出作答情况、整理收藏集，以及查看和管理网站写入浏览器的数据。
- **JaLaQuery**：用 Rust 编写的命令行工具。读取导出的 JSON，按试卷统计正确率、估算 JLPT 分数、列出错题和答案解析，并支持跨试卷比较。

界面和命令行提示均为中文。所有数据只保存在本机，不上传到其他服务。

> 当前状态：用户脚本与命令行工具已实现。用户脚本在 Chromium 中完成了单题保存、整套抓取、收藏集导出和存储管理的测试；Firefox + Tampermonkey 的实际安装需要手动验证。命令行工具已通过单元测试和端到端导入测试。

## 调研结论

开发前对网站的页面数据、浏览器存储和答题行为做了检查，结论如下。

### 作答记录存在哪里

网站的每道题会在 `localStorage` 中写入一个计数键：

```text
jlpt:stats:<级别>/<试卷>/<类别>/<章节>/<页码>
```

值只有正确和错误数量，例如：

```json
{"correct": 1, "wrong": 0}
```

一次“对答案”会按当前页的小题数量累加计数。一个页面包含多个小题时，正确和错误分别按小题累加。

网站**不保存每次作答的选项、时间或完整历史**，也没有在点击“对答案”时向服务器提交作答记录。检查浏览器网络请求后，点击“对答案”只修改 `localStorage`，没有站点接口请求。

因此：

- 用户当前选择的选项只能由脚本在题目页面读取。
- 网站自己的记录只能提供正确和错误数量。
- 由脚本保存的每条记录才是完整、可追溯的作答历史。

### 正确答案与解析

题目的原始数据在页面内嵌的 Next.js 数据流中，包含：

- 题干、选项、正确答案。
- 阅读文章、听力原文。
- 答案解析。

正确答案在未点击“对答案”时也能从页面数据中读取。答案解析和听力原文在未登录时是占位文本（`Lorem ipsum`），页面上有“登录后查看”遮罩。登录后网站应返回真实内容，但本次开发环境没有登录状态，未能实际验证。脚本不会绕过登录限制：读到真实内容就保存，读到占位内容则标记为“未解锁”，不会把占位文本当成解析。

### 其他存储键

检查到的站点键：

| 键 | 内容 |
| --- | --- |
| `jlpt:stats:*` | 每页正确、错误数量 |
| `jlpt:lastQuestion` | 最近打开的题目 |
| `jlpt:explanationVisible` | 解析是否展开 |
| `jlpt:transcriptVisible` | 原文是否展开 |
| `jlpt-audio-mode` | 音频播放模式，位于 `sessionStorage` |
| `jala:floating-clock` | 网站自带的浮动时钟设置 |

### 题目结构

一套试卷的页面按“类别 / 章节 / 页码”组织。一个页面可能包含多个小题：

- N2 2020 年 12 月共 88 个页面、102 个小题。
- 语言知识 51 小题，阅读 21 小题，听力 30 小题。
- 计分应以小题为单位，不能按页面数计算。

脚本会同时保存页面数据和每个小题的选项、正确答案、解析。

## 技术选型

选择 **Tampermonkey 用户脚本**，不开发 Firefox 扩展。

| 方案 | 优点 | 缺点 |
| --- | --- | --- |
| 用户脚本 | 单文件、无需构建和签名；与页面同源，可直接读写 `localStorage` 和页面数据；安装、修改、调试都简单 | 需要安装 Tampermonkey；`localStorage` 容量约 5 MB |
| Firefox 扩展 | 存储空间更大，界面与页面隔离更彻底 | 需要清单文件、构建和签名；常驻安装比用户脚本麻烦；本项目只有一个站点，隔离带来的收益有限 |

项目是一个自用的小工具，用户脚本更合适。脚本使用 `@grant none` 运行在页面上下文中，直接读取页面内嵌数据。

## 安装用户脚本

1. 在 Firefox 中安装 [Tampermonkey](https://addons.mozilla.org/firefox/addon/tampermonkey/)。
2. 打开 Tampermonkey 管理面板，选择“新建脚本”。
3. 用 `extension/jalaplus.user.js` 的内容替换编辑器中的模板，按 `Ctrl+S` 保存。
4. 打开任意 `https://jala.tools/zh/jlpt/...` 页面，右侧下方会出现三个圆形按钮。

脚本只匹配 `https://jala.tools/<语言>/jlpt/...`，不会在其他页面运行。

## 使用用户脚本

右侧下方有三个按钮：

- **加号**：在题目页面保存当前题目；在试卷页面抓取整套试卷。
- **收藏集**：查看、筛选、导出和删除已保存的题目记录。
- **存储**：查看和管理 `localStorage`、`sessionStorage` 中的站点数据。

### 保存一道题

打开一道题，选择选项后点击加号。记录包含：

- 题干、选项、正确答案。
- 当前选择的选项（如果已选择）。
- 答案解析（登录后且网站返回真实内容时）。
- 阅读文章、听力原文、音频地址。
- 网站的 `jlpt:stats` 计数。
- 保存时间、来源、页面地址。

如果当前没有选择任何选项，仍会保存题目和正确答案，但不记录用户作答，并显示提示。

### 抓取整套试卷

打开试卷页面（例如 `https://jala.tools/zh/jlpt/n2/2020-12`），点击加号。脚本会：

1. 读取页面上各题型的题量。
2. 逐页请求题目页面，解析题干、选项、正确答案、解析和听力原文。
3. 读取每道题的网站计数。
4. 用同一个抓取批次编号保存所有记录。

抓取过程显示进度，可以取消。请求失败或遇到人机验证时会提示，并保留已成功的部分。

### 收藏集

收藏集默认按试卷查看：

- 当前试卷展开，其他试卷折叠。
- 可以切换到按题型查看；没有当前题目时展开第一个题型。
- 每道题显示最新一条记录的作答情况；展开后可看到该题的所有记录。
- 题目的勾选用于导出和删除；抓取批次编号用于快速选中同一次抓取的全部记录，也可以单独导出或删除该批记录。

可执行的操作：

- 导出单题、多题、整套试卷、多套试卷、一个题型、多个题型。
- 删除单条记录、多选删除、按抓取批次删除。
- 导出格式为 JSON，文件名包含导出范围和日期。

“删除本批”只删除该抓取批次产生的记录；“删除所选”会删除所选题目下的全部记录，包括其他批次。导出时，所选题目的所有记录都会包含在内；只导出某一批时使用“导出本批”。

### 存储管理

“存储”面板列出 `localStorage` 和 `sessionStorage` 的键、大小和内容预览，可以：

- 查看完整内容。
- 删除单个键。
- 清除网站作答统计（`jlpt:stats:*`）。
- 清除站点数据（当前站点除 `jalaplus:*` 外的键）。
- 导出当前存储内容为 JSON。

删除前会二次确认。脚本不会删除 Cookie，也不会读取登录凭据。

## 导出格式

导出文件是一个 JSON 对象，顶层字段：

```json
{
  "format": "jalaplus.export",
  "version": 1,
  "exportedAt": "2026-10-20T10:00:00.000Z",
  "scope": {
    "kind": "selection",
    "description": "N2 2020年12月 · 构词 第1题"
  },
  "records": []
}
```

每条记录的主要字段：

```json
{
  "id": "记录编号",
  "capturedAt": "保存时间",
  "source": "question 或 exam",
  "batchId": "抓取批次编号，单题保存时为 null",
  "paper": {
    "level": "n2",
    "exam": "2020-12",
    "title": "JLPT N2・2020年12月",
    "locale": "zh",
    "pageCount": 88,
    "chapterPages": {
      "vocabulary/word-formation": 3
    },
    "groupCount": 102,
    "chapterGroups": {
      "vocabulary/word-formation": 3
    },
    "complete": true
  },
  "question": {
    "category": "vocabulary",
    "chapter": "word-formation",
    "index": 1,
    "url": "https://jala.tools/zh/jlpt/n2/2020-12/vocabulary/word-formation/1",
    "labels": {
      "category": "词汇",
      "chapter": "构词"
    },
    "prompt": "题干",
    "passage": {
      "text": "阅读文章",
      "translation": "当前语言的翻译"
    },
    "audio": "/jlpt/audio/...",
    "transcript": {
      "roles": [],
      "sentences": [
        {
          "startMs": 0,
          "endMs": 4000,
          "text": "听力原文"
        }
      ]
    },
    "groups": [
      {
        "id": "g1",
        "prompt": "小题题干",
        "options": [
          {
            "id": "a",
            "label": "选项内容"
          }
        ],
        "answer": "c",
        "explanation": "小题解析"
      }
    ],
    "explanation": "整题解析"
  },
  "attempt": {
    "answers": {
      "g1": "c"
    },
    "answeredGroups": 1,
    "totalGroups": 1,
    "checked": true,
    "siteStats": {
      "correct": 1,
      "wrong": 0
    }
  },
  "access": {
    "explanation": "full、locked 或 absent",
    "transcript": "full、locked 或 absent"
  }
}
```

说明：

- `question.groups` 中的每个小题是一个计分单位。一个页面可能有多个小题。
- `attempt.answers` 只包含用户实际选择的选项。未作答时为 `null`。
- `access` 标记解析和原文是否被登录限制遮挡。`locked` 表示网站只返回了占位内容。
- `siteStats` 是网站自己的计数，不是本次记录的选择结果。计数按小题累加。
- `paper.pageCount`、`chapterPages` 记录整套抓取时的页面数量；`paper.groupCount`、`chapterGroups` 记录小题数量；`complete` 表示这次抓取是否覆盖整套试卷。单题保存时这些字段为 `null`。

## 命令行工具 JaLaQuery

JaLaQuery 读取导出的 JSON，保存到本机数据文件，之后可以反复查询。

### 安装

```bash
cd cli
cargo test
cargo build --release
mkdir -p ~/.local/bin
cp target/release/jalaquery ~/.local/bin/
```

也可以运行 `cargo install --path cli`，安装到 Cargo 的可执行文件目录。

默认数据文件：

```text
$XDG_DATA_HOME/jalaquery/store.json
```

未设置 `XDG_DATA_HOME` 时使用 `~/.local/share/jalaquery/store.json`。可以用 `--store <路径>` 或环境变量 `JALAQUERY_STORE` 指定其他位置。

### 导入

```bash
jalaquery import ~/Downloads/jalaplus-n2-2020-12-*.json
```

同一个记录编号重复导入时会覆盖旧记录，不会产生重复数据。

### 常用命令

```bash
# 各试卷的作答数量、正确率和最近保存时间
jalaquery overview

# 一套试卷的估算分数、各题型正确率和错题
jalaquery paper n2/2020-12

# 只要分数，不要 JSON 里的错题详情
jalaquery paper n2/2020-12 --no-wrong --json

# 错题、用户答案、正确答案和解析
jalaquery wrong n2/2020-12 --limit 20
jalaquery wrong n2/2020-12 --type word-formation --json
jalaquery wrong n2/2020-12 --all --no-explanation

# 某道题的全部记录
jalaquery show n2/2020-12/vocabulary/word-formation/1

# 按题型、类别或试卷统计正确率
jalaquery stats --by chapter
jalaquery stats n2/2020-12 --by category
jalaquery stats --by paper

# 比较两套或多套试卷
jalaquery compare n2/2020-07 n2/2020-12
```

给程序或 Agent 使用时加 `--json`，输出紧凑 JSON；加 `--pretty` 输出带缩进的 JSON。JSON 字段使用 camelCase，例如 `expectedGroups`、`answeredGroups`、`totalScore`、`passLine`。

### 判分方式

JLPT 官方使用尺度得点，具体算法没有公开。JaLaQuery 只能按题目固定分值估算：

- N1、N2、N3：语言知识 60 分、阅读 60 分、听力 60 分。
- N4、N5：语言知识与阅读合计 120 分、听力 60 分。
- 每个得点区分内，按小题数量平均分配分数。
- 总分 180 分，合格线和基准点按官方公布的标准：

| 级别 | 合格点 | 语言知识 | 阅读 | 听力 |
| --- | --- | --- | --- | --- |
| N1 | 100 | 19 | 19 | 19 |
| N2 | 90 | 19 | 19 | 19 |
| N3 | 95 | 19 | 19 | 19 |
| N4 | 90 | 38（语言知识与阅读） | - | 19 |
| N5 | 80 | 38（语言知识与阅读） | - | 19 |

同一道题有多条记录时，判分使用最新一条有作答的记录。未作答的小题按 0 分计算，并在结果中显示覆盖率。

`paper.complete` 表示整套试卷是否完整抓取。只有完整抓取时，`passed` 才会给出合格或不合格；抓取不完整时 `passed` 为 `null`，分数只覆盖已记录的小题。

### 输出示例

```text
试卷：JLPT N2・2020年12月
覆盖：96 / 102 小题（94.1%），88 / 88 个页面
估算总分：120 / 180（合格线 90，合格）

得点区分：
  语言知识：40 / 60（基准点 19；已答 48/51，正确 34，正确率 70.8%）
  阅读：40 / 60（基准点 19；已答 20/21，正确 14，正确率 70.0%）
  听力：40 / 60（基准点 19；已答 28/30，正确 20，正确率 71.4%）
```

## Skill

`skills/jalaquery/SKILL.md` 是给 LLM 使用的技能说明，包含命令选择、参数示例和节省 token 的建议。

安装到 omp 的用户级技能目录：

```bash
mkdir -p ~/.omp/agent/skills
cp -r skills/jalaquery ~/.omp/agent/skills/
```

也可以放在项目的 `.omp/skills/jalaquery/` 下，只在该项目内生效。重启 omp 后可用 `/skill:jalaquery` 查看说明。

Claude Code 用户可复制到 `~/.claude/skills/jalaquery/`。

技能默认调用 `PATH` 中的 `jalaquery`。如果二进制不在 `PATH`，先安装到 `~/.local/bin` 或修改技能中的命令路径。

## 已知限制

- 网站只在 `localStorage` 保存正确和错误数量，不保存每次选择的选项和时间。完整历史只能由脚本从安装后开始记录。
- 网站的正确、错误计数按小题累加，不能还原出用户选择了哪个选项。脚本保存的 `attempt.answers` 才是本次选择。
- 解析和听力原文需要登录。未登录时保存的记录会标记为 `locked`，不会保存占位文本。登录后网站返回的真实内容尚未在本开发环境中验证。
- 网站返回的解析可能是当前语言的一份文本。脚本按当前页面语言取值，缺失时回退到中文、英文。
- `localStorage` 容量通常约 5 MB。包含阅读文章和听力原文的记录会占用较多空间，收藏集接近上限时会提示删除旧记录。实测一套 N2 试卷的 88 条记录约占用 120 KB。
- JLPT 官方分数使用尺度得点，命令行只能给出等分估算，不等同于真实考试成绩。
- 网站页面结构变化可能导致解析失败。脚本会优先读取 Next.js 页面数据，读取失败时给出提示，不会保存空记录。
- 当前沙箱中 Firefox 无法启动（用户命名空间创建失败），用户脚本的页面测试使用 Chromium 完成。Firefox + Tampermonkey 的实际安装需要手动验证。

## 开发与测试

用户脚本是单个 ES2020 文件，无需构建。命令行工具使用 Rust 2021。

```bash
# 命令行测试：单元测试 + 端到端导入/查询
cd cli
cargo test

# 用户脚本语法检查
node --check extension/jalaplus.user.js
```

本次开发已完成以下验证：

- 在 Chromium 中注入用户脚本，验证单题保存、未作答提示、点击“对答案”后的状态、收藏集按试卷查看、按题型查看、导出所选、存储面板和整套抓取。
- 整套抓取 N2 2020 年 12 月：88 个页面全部成功，保存 88 条记录、102 个小题；抓取约 1 分钟。
- 用户脚本导出的真实 JSON 已通过 `jalaquery import` 导入，`overview`、`paper`、`wrong`、`show` 正常输出。
- Rust 单元测试覆盖得点区分、合格线、数据单位换算；集成测试覆盖导入、判分、错题、题目详情、题型统计和重复导入。

用户脚本的页面测试步骤：

1. 在 Chromium 中打开一道 JLPT 题目页面。
2. 通过开发者工具或自动化脚本注入 `extension/jalaplus.user.js`。
3. 检查三个浮动按钮、单题保存、收藏集导出、存储面板。
4. 在试卷页面检查整套抓取、进度和取消。

Firefox 手动测试步骤：

1. 安装 Tampermonkey 并导入脚本。
2. 打开题目页面，确认按钮出现。
3. 选择选项并保存，确认记录包含选择的选项和正确答案。
4. 登录后重复保存，确认解析和听力原文已保存；未登录时应标记为未解锁。
5. 打开试卷页面抓取整套，确认记录数量与页面题量一致。

## 目录结构

```text
.
├── README.md
├── extension/
│   └── jalaplus.user.js
├── cli/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── model.rs
│   │   ├── report.rs
│   │   ├── score.rs
│   │   └── store.rs
│   └── tests/
│       └── cli.rs
└── skills/
    └── jalaquery/
        └── SKILL.md
```
