---
name: jalaquery
description: 查询和分析 JaLaPlus 导出的 JLPT 作答记录，包括估算判分、错题与解析、正确率和跨试卷对比。用户询问 JLPT 练习成绩、错题、某题作答历史或需要分析 jalaplus 导出 JSON 时使用。
---

# JaLaQuery

JaLaQuery 读取 JaLaPlus 用户脚本导出的 JSON，保存在本机数据文件中，再用命令查询。它不访问网络，不调用网站接口。

## 前提

- `jalaquery` 已在 `PATH` 中。
- 用户已经把导出的 JSON 文件放在本机。没有文件时，先让用户在 jala.tools 的收藏集里导出，不要猜测数据。

默认数据文件：

```text
$XDG_DATA_HOME/jalaquery/store.json
```

未设置 `XDG_DATA_HOME` 时是 `~/.local/share/jalaquery/store.json`。可用 `--store <文件>` 指定其他文件，后续命令要使用同一个 `--store`。

## 数据单位

一条记录对应网站上的一个题目页面。一个页面可能包含多个小题，字段是 `question.groups`。判分和正确率以小题为单位，页面只用于定位。

`paper` 字段表示一套试卷，例如 `n2/2020-12`。题型由 `question.category` 和 `question.chapter` 共同确定。

## 标准流程

### 1. 导入

```bash
jalaquery import ~/Downloads/jalaplus-*.json --json
```

也可以传入目录，导入目录下的全部 JSON 文件。重复导入会按记录编号更新，不会产生重复记录。

### 2. 先看总览

```bash
jalaquery overview --json
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

### 3. 看一套试卷的分数和错题概览

```bash
jalaquery paper n2/2020-12 --json
```

重点字段：

- `totalScore`、`maxScore`、`passLine`、`passed`
- `sections`：每个得点区分的 `expected`、`answered`、`correct`、`score`、`max`、`min`、`pointsPerGroup`、`accuracy`
- `chapters`：每个题型的正确率
- `wrong`：错题数组；如果只需要分数，加 `--no-wrong`

`passed` 只有在 `complete` 为 `true` 时才给出布尔值；抓取不完整时是 `null`，不要把它解释成合格或不合格。

### 4. 看错题和解析

```bash
jalaquery wrong n2/2020-12 --json --limit 20
```

每个错题包含：

- `paper`、`category`、`categoryLabel`、`chapter`、`chapterLabel`、`index`
- `group`、`prompt`
- `user`、`answer`：各有 `id`、`label`、`number`
- `explanation`

只要错题、不要解析时加 `--no-explanation`。只要一个题型时加 `--type`：

```bash
jalaquery wrong n2/2020-12 --type word-formation --json
jalaquery wrong n2/2020-12 --type 词汇/构词 --json
```

默认最多 20 道；需要全部时加 `--all`。

### 5. 看某道题的全部记录

```bash
jalaquery show n2/2020-12/vocabulary/word-formation/1 --json
```

也可以按记录编号：

```bash
jalaquery show <记录编号> --json
```

返回该题的所有记录，按保存时间从新到旧。用于查看用户在不同时间的选择、正确答案、解析和听力原文。除非用户明确问到某道题，不要先调用这个命令，因为它返回的内容较多。

### 6. 按题型或类别统计

```bash
jalaquery stats n2/2020-12 --by chapter --json
jalaquery stats n2/2020-12 --by category --json
jalaquery stats --by paper --json
```

`--by chapter` 是题型级别，`--by category` 是文字、词汇、语法、阅读、听力级别。

### 7. 跨试卷比较

```bash
jalaquery compare n2/2020-07 n2/2020-12 --json
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

回答用户时必须说明这是按题目等分的估算，不是官方尺度得点，也不等同于真实考试成绩。

## 节省 token 的建议

- 优先用 `--json`，不要用 `--pretty`。
- 先 `overview`，再 `paper`，最后才 `wrong` 或 `show`。
- 用 `--limit` 控制错题数量；只有在需要全部错题时才用 `--all`。
- `paper --json` 已经包含错题摘要；不需要重复调用 `wrong`，除非需要解析。
- 不要一次导出整个数据文件到对话里。

## 常见错误

- `没有可查询的记录`：数据文件为空，先让用户导出并 `import`。
- `没有找到试卷 n2/2020-12 的记录`：检查试卷编号，或用 `overview` 看已有试卷。
- `导出文件格式不正确`：确认文件来自 JaLaPlus 的收藏集导出，顶层应包含 `format: "jalaplus.export"` 和 `records`。
- `passed` 为 `null`：抓取不完整，无法判断合格。不要补造结论。

## 安全边界

- 不要在没有用户明确要求时运行 `jalaquery clear --yes`。
- 不要修改数据文件。
- 不要编造记录、分数或解析。数据里没有的内容就直说没有。
