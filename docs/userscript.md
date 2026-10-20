# 用户脚本

`extension/jalaplus.user.js` 运行在 `https://jala.tools/*/jlpt*` 页面，负责读取题目、保存记录和导出 JSON。

匹配规则：

```text
https://jala.tools/*/jlpt
https://jala.tools/*/jlpt/*
```

## 网站数据

### 作答计数

每道题会在 `localStorage` 中写入：

```text
jlpt:stats:<级别>/<试卷>/<类别>/<章节>/<页码>
```

值只有正确和错误数量：

```json
{"correct": 1, "wrong": 0}
```

一次“对答案”按页面内的小题数量累加计数。多小题页面只保存该页合计，无法还原具体小题的选择。

### 题目内容

题目数据位于页面内嵌的 Next.js 数据流，包含：

- 题干、选项、正确答案。
- 阅读文章、听力原文、音频地址。
- 答案解析。

未登录时，解析和听力原文可能是占位文本（`Lorem ipsum`），页面上有登录遮罩。脚本读到占位文本时把对应 `access` 标记为 `locked`。

### 其他站点键

| 键 | 内容 |
| --- | --- |
| `jlpt:lastQuestion` | 最近打开的题目 |
| `jlpt:explanationVisible` | 解析是否展开 |
| `jlpt:transcriptVisible` | 原文是否展开 |
| `jlpt-audio-mode` | 音频播放模式，位于 `sessionStorage` |
| `jala:floating-clock` | 网站自带的浮动时钟设置 |

### 题目结构

一套试卷按“类别 / 章节 / 页码”组织，一个页面可能包含多个小题。N2 2020 年 12 月共有 88 个页面、102 个小题：

- 语言知识 51 小题。
- 阅读 21 小题。
- 听力 30 小题。

计分以小题为单位。

## 存储结构

| 数据 | 位置 |
| --- | --- |
| 题目记录、批次、连续答题会话 | IndexedDB `jalaplus` |
| 连续答题开关、当前标签页会话编号 | `sessionStorage` |
| 网站自身数据 | `localStorage` |

IndexedDB 对象仓库：

| 仓库 | 主键 | 内容 |
| --- | --- | --- |
| `records` | `id` | 单题、整套抓取、连续作答记录 |
| `batches` | `id` | 整套抓取批次 |
| `sessions` | `id` | 连续答题会话 |
| `meta` | `key` | 版本等少量元数据 |

每条记录带 `paperRef`、`questionRef`、`batchId`、`sessionId`，用于查询和合并。

## 连续答题

点击绿色播放按钮开始，红色停止按钮结束。状态保存在当前标签页的 `sessionStorage`，刷新页面后恢复；关闭标签页后会话结束。会话开始和结束时间写入 IndexedDB。

点击网站“对答案”时自动记录：

```text
pageMs    = 点击时间 - 当前页面进入时间
sessionMs = 点击时间 - 连续答题开始时间
```

页面进入时间：

- 首次载入：文档导航开始时间。
- 客户端路由切换：路由变化时间。

记录中的 `timing`：

```json
{
  "sessionId": "会话编号",
  "pageLoadedAt": "2026-10-20T09:59:50.000Z",
  "answeredAt": "2026-10-20T10:00:00.000Z",
  "pageMs": 10000,
  "sessionStartedAt": "2026-10-20T09:58:00.000Z",
  "sessionMs": 120000,
  "navigation": "spa"
}
```

客户端路由通过 `location` 轮询同步；脚本没有修改 `history`。Vimium 等工具 fork 标签页时会复制 `sessionStorage`，两个页面可能共享同一会话编号；当前会话结束时间以最后一次写入为准。

## 导出格式

顶层字段：

```json
{
  "format": "jalaplus.export",
  "version": 2,
  "generator": "JaLaPlus 0.2.0",
  "exportedAt": "2026-10-20T10:00:00.000Z",
  "scope": {"kind": "selection", "description": "已选 16 道题"},
  "records": [],
  "batches": [],
  "sessions": []
}
```

记录按 `kind` 区分：

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
    "groups": [
      {
        "id": "g1",
        "prompt": "",
        "options": [{"id": "a", "label": "选项"}],
        "answer": "a",
        "explanation": "小题解析",
        "audio": null
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
  "access": {"explanation": "full", "transcript": "absent"}
}
```

说明：

- `question.groups` 的每个小题是计分单位。
- `attempt.answers` 只包含用户实际选择的选项。
- `siteStats` 是网站累计计数，不是本次选择。
- `batches` 保存整套抓取的页数、小题数、章节分布和完整状态。
- `sessions` 保存连续答题会话的开始和结束时间。

## 已知限制

- 网站计数按小题累加，多小题页面只有合计。
- 解析和听力原文可能需要登录。
- 网站页面结构变化可能导致解析失败。
- Vimium fork 的多个标签页可能共享同一个连续答题会话。
