# 命令行工具

JaLaQuery 用 Rust 编写，直接读取 JaLaPlus 导出的 JSON。每次命令在内存中读取输入、按记录编号去重、按试卷和题型建立索引，不创建本地数据库。

## 安装

```bash
cd cli
cargo build --release
mkdir -p ~/.local/bin
cp target/release/jalaquery ~/.local/bin/
```

也可以在 `cli/` 目录运行 `cargo install --path .`。

## 输入

`--input` 接受一个或多个 JSON 文件或目录，可以重复：

```bash
jalaquery --input export.json overview
jalaquery --input exports/ paper n2/2020-12
jalaquery --input a.json --input b.json compare
```

目录中的 `jalaplus.storage` 存储备份会跳过，分析使用 `jalaplus.export` 记录导出。

## 命令

```bash
# 各试卷的作答数量、正确率和最近保存时间
jalaquery --input export.json overview

# 一套试卷的估算分数、题型正确率和错题
jalaquery --input export.json paper n2/2020-12

# 同时查看网站累计计数
jalaquery --input export.json paper n2/2020-12 --site-stats

# 错题、用户答案、正确答案和解析
jalaquery --input export.json wrong n2/2020-12 --limit 20
jalaquery --input export.json wrong n2/2020-12 --type word-formation --json

# 使用网站累计计数查看错误页面
jalaquery --input export.json wrong n2/2020-12 --site-stats

# 某道题的全部记录
jalaquery --input export.json show n2/2020-12/vocabulary/word-formation/1
jalaquery --input export.json show <记录编号>

# 按题型、类别或试卷统计正确率
jalaquery --input export.json stats --by chapter
jalaquery --input export.json stats n2/2020-12 --by category
jalaquery --input export.json stats --by paper
jalaquery --input export.json stats --by chapter --site-stats

# 连续答题时间统计
jalaquery --input export.json timing
jalaquery --input export.json timing n2/2020-12 --json

# 比较两套或多套试卷
jalaquery --input a.json --input b.json compare
```

`--json` 输出紧凑 JSON，`--pretty` 输出带缩进的 JSON；JSON 字段使用 camelCase。

## 判分

JLPT 官方使用尺度得点，具体算法没有公开。JaLaQuery 按小题固定分值估算：

- N1、N2、N3：语言知识 60 分、阅读 60 分、听力 60 分。
- N4、N5：语言知识与阅读合计 120 分、听力 60 分。
- 每个得点区分内，按小题数量平均分配分数。
- 总分 180 分，合格线和基准点取自官方公布的标准：

| 级别 | 合格点 | 语言知识 | 阅读 | 听力 |
| --- | --- | --- | --- | --- |
| N1 | 100 | 19 | 19 | 19 |
| N2 | 90 | 19 | 19 | 19 |
| N3 | 95 | 19 | 19 | 19 |
| N4 | 90 | 38（语言知识与阅读） | - | 19 |
| N5 | 80 | 38（语言知识与阅读） | - | 19 |

同一道题有多条作答记录时，使用最新一条有作答的记录。未作答的小题按 0 分计算，并显示覆盖率。

`complete` 来自整套抓取批次。只有完整抓取时，`passed` 才给出合格或不合格；没有完整抓取信息时 `passed` 为 `null`，分数只覆盖已记录的作答。

## 输出字段

`overview`：

- `paper`、`title`
- `capturedPages`、`pageCount`
- `expectedGroups`、`answeredGroups`
- `correctGroups`、`wrongGroups`
- `accuracy`
- `complete`
- `lastCapturedAt`

`paper`：

- `totalScore`、`maxScore`、`passLine`、`passed`
- `sections`：`expected`、`answered`、`correct`、`score`、`max`、`min`、`pointsPerGroup`、`accuracy`
- `chapters`：各题型覆盖和正确率
- `wrong`：错题；`--no-wrong` 可省略
- `siteStats`：加 `--site-stats` 后返回网站累计计数

`timing`：

- `count`、`sessions`
- `pageMs`、`sessionMs`：各自的 `mean`、`median`、`min`、`max`
- `rows`：每题的时间、导航方式和正确数

## Skill

`.agents/skills/jalaquery/SKILL.md` 是给 LLM 使用的命令说明，遵循 [Agent Skills](https://agentskills.io) 约定。项目级使用时，支持该约定的 agent 会从项目根目录发现它。

用户级安装：

```bash
mkdir -p ~/.agents/skills
cp -r .agents/skills/jalaquery ~/.agents/skills/
```

若使用的 agent 只扫描自己的技能目录，把同一目录复制到其文档指定位置；例如 omp 可复制到 `~/.omp/agent/skills/`。

## 已知限制

- 网站计数是累计值，不能还原具体选择。
- 官方尺度得点算法没有公开，命令行只提供等分估算。
- 同一套试卷没有完整抓取批次时，`passed` 无法判断。
