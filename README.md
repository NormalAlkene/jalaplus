# JaLaPlus

JaLaPlus 是一套自用的 JLPT 真题作答记录工具，由两部分组成：

- **JaLaPlus**：运行在 [jala.tools](https://jala.tools) 的 Tampermonkey 用户脚本。保存题目、抓取整套试卷、连续记录作答、管理收藏集并导出 JSON。
- **JaLaQuery**：Rust 命令行工具。直接读取一个或多个导出 JSON，按试卷统计正确率、估算 JLPT 分数、列出错题、比较试卷，并分析连续答题用时。

界面和命令行提示均为中文。数据保存在本机浏览器和本地文件中，不上传到其他服务。

> 当前状态：用户脚本与命令行工具已实现。用户脚本在 Chromium 中完成了单题保存、整套抓取、连续答题、收藏集和存储管理的测试；Firefox + Tampermonkey 的实际安装需要手动验证。

## 网站数据

### 作答计数

网站的每道题会在 `localStorage` 中写入一个计数键：

```text
jlpt:stats:<级别>/<试卷>/<类别>/<章节>/<页码>
```

值只有正确和错误数量：

```json
{"correct": 1, "wrong": 0}
```

一次“对答案”会按当前页的小题数量累加计数。一个页面包含多个小题时，正确和错误分别按小题累加。

网站按页面键保存计数，多小题页面只能得到该页的合计。网站没有保存每次作答的选项、时间或完整历史，点击“对答案”只修改 `localStorage`，没有提交作答记录。

完整、可追溯的作答历史由脚本记录。

### 题目内容

题目的原始数据在页面内嵌的 Next.js 数据流中，包含题干、选项、正确答案、阅读文章、听力原文和答案解析。

未登录时，解析和听力原文可能是占位文本（`Lorem ipsum`），页面上有登录遮罩。脚本读到真实内容就保存；读到占位文本时标记为 `locked`，保留网站的真实状态。

### 其他站点键

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
- 计分以小题为单位。

## 技术选型

采用 **Tampermonkey 用户脚本**：

- 单文件、无需构建和签名。
- 与页面同源，可直接读取页面数据。
- 安装、修改和调试简单。
- `@grant none` 运行在页面上下文，不申请额外权限。

记录存储在 **IndexedDB**，每套试卷和每次作答单独写入，避免单个 `localStorage` 键反复全量重写。`sessionStorage` 保存当前标签页的连续答题状态；`localStorage` 只保留网站自身的数据。

## 安装用户脚本

1. 在 Firefox 中安装 [Tampermonkey](https://addons.mozilla.org/firefox/addon/tampermonkey/)。
2. 打开 Tampermonkey 管理面板，选择“新建脚本”。
3. 用 `extension/jalaplus.user.js` 的内容替换编辑器中的模板，按 `Ctrl+S` 保存。
4. 打开任意 `https://jala.tools/zh/jlpt/...` 页面，右侧下方会出现四个圆形按钮。

脚本匹配：

```text
https://jala.tools/*/jlpt
https://jala.tools/*/jlpt/*
```

## 使用用户脚本

右侧下方有四个按钮：

- **加号**：在题目页面保存当前题目；在试卷页面抓取整套试卷。
- **录制**：开始或结束连续答题。
- **收藏集**：查看、筛选、导出和删除记录。
- **存储**：查看和管理 `localStorage`、`sessionStorage` 中的站点数据。

### 保存一道题

打开一道题，选择选项后点击加号。记录包含：

- 题干、选项、正确答案。
- 当前选择的选项（如果已选择）。
- 答案解析。
- 阅读文章、听力原文、音频地址。
- 网站的 `jlpt:stats` 计数。
- 保存时间、来源、页面地址。

如果当前没有选择任何选项，仍会保存题目和正确答案，不记录用户作答，并显示提示。

### 抓取整套试卷

打开试卷页面（例如 `https://jala.tools/zh/jlpt/n2/2020-12`），点击加号。脚本会：

1. 读取页面上各题型的题量。
2. 逐页请求题目页面，解析题干、选项、正确答案、解析和听力原文。
3. 读取每道题的网站计数。
4. 用同一个批次编号逐条写入 IndexedDB。
5. 记录批次的总页数、小题数和完整状态。

抓取过程显示进度，可以取消。请求失败或遇到人机验证时会提示，并保留已成功的部分。

### 连续答题

点击录制按钮开始连续答题：

- 按钮高亮，提示“结束连续答题”。
- 状态保存在当前标签页的 `sessionStorage`，刷新页面后恢复。
- 关闭标签页后，当前连续答题会话结束。
- 开始状态、会话编号和开始时间写入 IndexedDB 的会话记录。

开启后，每次点击网站的“对答案”按钮，脚本自动保存一次作答，包含：

```text
答案
是否正确
页面用时：从当前页面进入至点击“对答案”
会话用时：从本次连续答题开始至点击“对答案”
页面来源：document 或 spa
```

计时在点击“对答案”的瞬间定格。网站使用客户端路由，切题后页面用时从新题进入时重新计算；刷新页面后会话用时继续累计。

再次点击录制按钮结束连续答题，已保存的记录保留。

### 收藏集

收藏集默认按试卷查看：

- 当前试卷展开，其他试卷折叠。
- 切换到按题型查看时，优先展开当前题型。
- 每道题显示最新一条记录的作答情况；展开后可看到该题的所有记录。
- 连续答题记录显示页面用时和会话用时。
- 题目的勾选用于导出和删除；抓取批次编号用于快速选中同一次抓取的全部记录。

可执行的操作：

- 导出单题、多题、整套试卷、多套试卷、一个题型、多个题型、单个抓取批次。
- 删除单条记录、多选删除、按抓取批次删除。
- 导出格式为 JSON，文件名包含导出范围和日期。

“删除本批”删除该抓取批次产生的记录；“删除所选”删除所选题目下的全部记录，包括其他批次。

### 存储管理

“存储”面板列出 `localStorage` 和 `sessionStorage` 的键、大小和内容预览，并显示 IndexedDB 中的收藏记录数量。可以：

- 查看完整内容。
- 删除单个键。
- 清除网站作答统计（`jlpt:stats:*`）。
- 清除站点数据（当前站点除 `jalaplus:*` 外的键）。
- 导出当前 `localStorage`/`sessionStorage` 内容为 JSON。

删除前会二次确认。脚本不读取 Cookie 或登录凭据。

## 导出格式

顶层字段：

```json
{
  "format": "jalaplus.export",
  "version": 2,
  "generator": "JaLaPlus 0.2.0",
  "exportedAt": "2026-10-20T10:00:00.000Z",
  "scope": {
    "kind": "selection",
    "description": "N2 2020年12月 · 构词 第1题"
  },
  "records": [],
  "batches": [],
  "sessions": []
}
```

每条记录使用 `kind` 区分来源：

| `kind` | 含义 | `attempt` | `timing` |
| --- | --- | --- | --- |
| `paper` | 整套抓取 | `null` | `null` |
| `question` | 单题保存 | 有选择时存在 | `null` |
| `answer` | 连续答题 | 必有 | 必有 |

记录示例：

```json
{
  "id": "记录编号",
  "kind": "answer",
  "capturedAt": "2026-10-20T10:00:00.000Z",
  "batchId": null,
  "sessionId": "会话编号",
  "paperRef": "n2/2020-12",
  "questionRef": "n2/2020-12/vocabulary/word-formation/1",
  "paper": {
    "level": "n2",
    "exam": "2020-12",
    "title": "JLPT N2・2020年12月",
    "locale": "zh"
  },
  "question": {
    "category": "vocabulary",
    "chapter": "word-formation",
    "index": 1,
    "prompt": "题干",
    "passage": null,
    "audio": null,
    "groups": [
      {
        "id": "g1",
        "prompt": "",
        "options": [
          {"id": "a", "label": "选项内容"}
        ],
        "answer": "a",
        "explanation": "小题解析"
      }
    ],
    "transcript": null,
    "explanation": "整题解析",
    "siteStats": {"correct": 1, "wrong": 0},
    "url": "https://jala.tools/zh/jlpt/n2/2020-12/vocabulary/word-formation/1",
    "labels": {"category": "词汇", "chapter": "构词"}
  },
  "attempt": {
    "answers": {"g1": "a"},
    "answeredGroups": 1,
    "totalGroups": 1,
    "checked": true
  },
  "timing": {
    "sessionId": "会话编号",
    "pageLoadedAt": "2026-10-20T09:59:50.000Z",
    "answeredAt": "2026-10-20T10:00:00.000Z",
    "pageMs": 10000,
    "sessionStartedAt": "2026-10-20T09:58:00.000Z",
    "sessionMs": 120000,
    "navigation": "spa"
  },
  "access": {
    "explanation": "full",
    "transcript": "absent"
  }
}
```

说明：

- `question.groups` 中的每个小题是一个计分单位。
- `attempt.answers` 只包含用户实际选择的选项。
- `siteStats` 是网站自己的累计计数，不是本次记录的选择结果。
- `access` 标记解析和原文是否被登录限制遮挡。
- `batches` 保存整套抓取的页数、小题数、章节分布和完整状态。
- `sessions` 保存连续答题会话的开始和结束时间。

## 命令行工具 JaLaQuery

JaLaQuery 直接读取导出 JSON，不依赖本地数据库。

### 安装

```bash
cd cli
cargo test
cargo build --release
mkdir -p ~/.local/bin
cp target/release/jalaquery ~/.local/bin/
```

也可以在 `cli/` 目录运行 `cargo install --path .`，安装到 Cargo 的可执行文件目录。

### 输入

`--input` 接受一个或多个 JSON 文件或目录，可以重复：

```bash
jalaquery --input ~/Downloads/jalaplus-n2-2020-12.json overview
jalaquery --input exports/ paper n2/2020-12
jalaquery --input n2-2020-07.json --input n2-2020-12.json compare
```

每次命令在内存中读取输入、按记录编号去重、按试卷和题型建立索引，输出结果后结束。需要保留历史时，保留导出文件或目录即可。

### 常用命令

```bash
# 各试卷的作答数量、正确率和最近保存时间
jalaquery --input export.json overview

# 一套试卷的估算分数、各题型正确率和错题
jalaquery --input export.json paper n2/2020-12

# 同时查看网站累计计数
jalaquery --input export.json paper n2/2020-12 --site-stats

# 错题、用户答案、正确答案和解析
jalaquery --input export.json wrong n2/2020-12 --limit 20
jalaquery --input export.json wrong n2/2020-12 --type word-formation --json

# 按网站累计计数查看错误页面
jalaquery --input export.json wrong n2/2020-12 --site-stats

# 某道题的全部记录
jalaquery --input export.json show n2/2020-12/vocabulary/word-formation/1

# 按题型、类别或试卷统计正确率
jalaquery --input export.json stats --by chapter
jalaquery --input export.json stats n2/2020-12 --by category

# 连续答题时间统计
jalaquery --input export.json timing
jalaquery --input export.json timing n2/2020-12 --json

# 比较两套或多套试卷
jalaquery --input n2-2020-07.json --input n2-2020-12.json compare
```

`--json` 输出紧凑 JSON，`--pretty` 输出带缩进的 JSON。JSON 字段使用 camelCase。

### 判分方式

JLPT 官方使用尺度得点，具体算法没有公开。JaLaQuery 按小题固定分值估算：

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

同一道题有多条作答记录时，判分使用最新一条有作答的记录。未作答的小题按 0 分计算，并显示覆盖率。

`complete` 来自整套抓取批次。只有完整抓取时，`passed` 才给出合格或不合格；没有完整抓取信息时 `passed` 为 `null`，分数只覆盖已记录的作答。

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

- 网站只在 `localStorage` 保存累计正确、错误数量，完整作答历史由脚本记录。
- 多小题页面的网站计数只有合计，无法还原具体小题的选择。
- 解析和听力原文需要登录。未登录时保存的记录标记为 `locked`。
- 网站页面结构变化可能导致解析失败。脚本优先读取 Next.js 页面数据，读取失败时给出提示。
- JLPT 官方分数使用尺度得点，命令行给出等分估算，不等同于真实考试成绩。
- 当前沙箱中 Firefox 无法启动（用户命名空间创建失败），用户脚本的页面测试使用 Chromium 完成。Firefox + Tampermonkey 的实际安装需要手动验证。

## 开发与测试

用户脚本是单个 ES2020 文件，无需构建。命令行工具使用 Rust 2021。

```bash
# 命令行测试：单元测试 + 端到端查询
cd cli
cargo test

# 用户脚本语法检查
node --check extension/jalaplus.user.js
```

页面测试步骤：

1. 在 Chromium 中打开一道 JLPT 题目页面。
2. 通过开发者工具或自动化脚本注入 `extension/jalaplus.user.js`。
3. 检查四个浮动按钮、单题保存、连续答题、收藏集、存储面板。
4. 在试卷页面检查整套抓取、进度、取消和批次记录。

Firefox 手动测试步骤：

1. 安装 Tampermonkey 并导入脚本。
2. 打开题目页面，确认按钮出现。
3. 选择选项并保存，确认记录包含选择的选项和正确答案。
4. 登录后重复保存，确认解析和听力原文已保存；未登录时标记为未解锁。
5. 打开连续答题，完成数题后检查收藏集中的页面用时和会话用时。
6. 打开试卷页面抓取整套，确认记录数量与页面题量一致。
