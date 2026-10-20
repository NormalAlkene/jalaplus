---
name: jalaquery
description: 查询和分析 JaLaPlus 导出的 JLPT 作答记录，包括估算判分、错题与解析、正确率、连续答题用时和跨试卷对比。用户询问 JLPT 练习成绩、错题、答题时间、某题作答历史或需要分析 jalaplus 导出 JSON 时使用。
---

# JaLaQuery

JaLaQuery 直接读取 JaLaPlus 用户脚本导出的 JSON，不访问网络，不调用网站接口，也不创建本地数据库。

## 前提

- `jalaquery` 已在 `PATH` 中。
- 用户已经把导出的 JSON 文件放在本机。没有文件时，先让用户在 jala.tools 的收藏集里导出，不要猜测数据。

## 数据单位

一条记录对应网站上的一个题目页面。一个页面可能包含多个小题，字段是 `question.groups`。判分和正确率以小题为单位，页面只用于定位。

`paperRef` 表示一套试卷，例如 `n2/2020-12`。题型由 `question.category` 和 `question.chapter` 共同确定。

记录使用 `kind` 区分：

- `paper`：整套抓取，只有题目内容和网站计数。
- `question`：单题保存，有选择时包含 `attempt`。
- `answer`：连续答题，包含 `attempt` 和 `timing`。

## 标准流程

### 输入

所有命令都通过 `--input` 读取一个或多个 JSON 文件、目录：

```bash
jalaquery --input ~/Downloads/jalaplus-*.json overview --json
```

可以重复 `--input`：

```bash
jalaquery --input a.json --input b.json compare --json
```

### 1. 先看总览

```bash
jalaquery --input export.json overview --json
```

返回每套试卷的：

- `paper`、`title`
- `capturedPages`、`pageCount`
- `expectedGroups`、`answeredGroups`
- `correctGroups`、`wrongGroups`
- `accuracy`（正确数 / 已答小题数）
- `complete`（是否完整抓取整套试卷）
- `lastCapturedAt`

如果用户问“我最近练得怎么样”，先看这里，再决定是否需要下钻。

### 2. 看一套试卷的分数和错题概览

```bash
jalaquery --input export.json paper n2/2020-12 --json
```

重点字段：

- `totalScore`、`maxScore`、`passLine`、`passed`
- `sections`：每个得点区分的 `expected`、`answered`、`correct`、`score`、`max`、`min`、`pointsPerGroup`、`accuracy`
- `chapters`：每个题型的正确率
- `wrong`：错题数组；如果只需要分数，加 `--no-wrong`
- `siteStats`：加 `--site-stats` 后返回网站累计计数

`passed` 只有在 `complete` 为 `true` 时才是布尔值；没有完整抓取信息时是 `null`。

### 3. 看错题和解析

```bash
jalaquery --input export.json wrong n2/2020-12 --json --limit 20
```

每个错题包含：

- `paper`、`category`、`categoryLabel`、`chapter`、`chapterLabel`、`index`
- `group`、`prompt`
- `user`、`answer`：各有 `id`、`label`、`number`
- `explanation`

只要错题、不要解析时加 `--no-explanation`。只要一个题型时加 `--type`：

```bash
jalaquery --input export.json wrong n2/2020-12 --type word-formation --json
jalaquery --input export.json wrong n2/2020-12 --type 词汇/构词 --json
```

默认最多 20 道；需要全部时加 `--all`。

如果整套抓取没有用户作答，但网站计数有错误，用：

```bash
jalaquery --input export.json wrong n2/2020-12 --site-stats --json
```

这种结果只有页面级正确、错误次数，不能还原具体选择。

### 4. 看某道题的全部记录

```bash
jalaquery --input export.json show n2/2020-12/vocabulary/word-formation/1 --json
```

也可以按记录编号：

```bash
jalaquery --input export.json show <记录编号> --json
```

返回该题的所有记录，按保存时间从新到旧。用于查看用户在不同时间的选择、正确答案、解析和听力原文。除非用户明确问到某道题，不要先调用这个命令。

### 5. 按题型或类别统计

```bash
jalaquery --input export.json stats n2/2020-12 --by chapter --json
jalaquery --input export.json stats n2/2020-12 --by category --json
jalaquery --input export.json stats --by paper --json
```

`--by chapter` 是题型级别，`--by category` 是文字、词汇、语法、阅读、听力级别。加 `--site-stats` 时使用网站累计计数。

### 6. 连续答题用时

```bash
jalaquery --input export.json timing --json
jalaquery --input export.json timing n2/2020-12 --json
```

返回：

- `count`、`sessions`
- `pageMs`、`sessionMs`：各自的 `mean`、`median`、`min`、`max`
- `rows`：每题的 `pageMs`、`sessionMs`、`navigation`、`answered`、`correct`

`pageMs` 是从页面进入到点击“对答案”的时间；`sessionMs` 是从连续答题开始到点击“对答案”的时间。

### 7. 跨试卷比较

```bash
jalaquery --input a.json --input b.json compare --json
```

返回每套试卷的总体正确率和得点区分正确率，以及相同题型的逐项对比。至少要两套试卷。

## 判分说明

JLPT 官方使用尺度得点，具体算法没有公开。JaLaQuery 只能按小题等分估算：

- N1、N2、N3：语言知识 60 分、阅读 60 分、听力 60 分。
- N4、N5：语言知识与阅读合计 120 分、听力 60 分。
- 每个得点区分内按小题数量平均分配分数。
- 未作答的小题按 0 分计算。
- 同一道题有多条记录时，使用最新一条有作答的记录。
- 官方合格点：N1 100、N2 90、N3 95、N4 90、N5 80；各得点区分另有基准点。

回答用户时说明这是按题目等分的估算，不是官方尺度得点。

## 节省 token 的建议

- 优先用 `--json`，不要用 `--pretty`。
- 先 `overview`，再 `paper`，最后才 `wrong`、`timing` 或 `show`。
- 用 `--limit` 控制错题数量；只有在需要全部错题时才用 `--all`。
- `paper --json` 已经包含错题摘要；不需要重复调用 `wrong`，除非需要解析。
- 不要一次导出整个数据文件到对话里。

## 常见错误

- `没有可查询的记录`：输入文件为空，先让用户导出。
- `没有找到试卷 n2/2020-12 的记录`：检查试卷编号，或用 `overview` 看已有试卷。
- `导出文件版本不受支持`：确认文件来自当前版本的 JaLaPlus 收藏集导出，顶层 `version` 应为 `2`。
- `passed` 为 `null`：没有完整抓取信息，无法判断合格。

## 安全边界

- 不要修改用户的导出文件。
- 不要编造记录、分数或解析。数据里没有的内容就直说没有。
