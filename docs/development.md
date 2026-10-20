# 开发

## 目录

```text
extension/jalaplus.user.js   用户脚本
cli/                         Rust 命令行工具
  src/model.rs               导出数据模型
  src/store.rs               输入读取、合并和排序
  src/score.rs               判分与统计
  src/report.rs              文本输出
  src/main.rs                命令入口
  tests/cli.rs               集成测试
skills/jalaquery/SKILL.md    LLM 技能说明
```

## 开发命令

```bash
# 用户脚本语法检查
node --check extension/jalaplus.user.js

# 命令行测试
cd cli
cargo test
cargo clippy --all-targets

# 发布构建
cargo build --release
```

## 用户脚本

- 单文件 ES2020，无构建步骤。
- 记录存储在 IndexedDB；`sessionStorage` 保存当前标签页的连续答题状态。
- 网站使用客户端路由，脚本通过 `location` 轮询同步当前题目和页面计时。
- 自动记录通过监听页面上的“对答案”按钮完成，不拦截页面行为。
- 页面解析优先读取 Next.js 数据流；失败时提示。

### API 约束

脚本使用标准 Web API：IndexedDB、`sessionStorage`、`fetch`、`DOMParser`、`performance`、`Blob` 和 `setInterval`。

IndexedDB 没有按数据库查询精确配额的标准接口；`navigator.storage.estimate()` 返回整个 origin 的近似值，因此没有实现空间预警。写入失败时保留已成功写入的记录并提示。

客户端路由没有可监听的 `pushState` 事件，路由同步使用 `location` 轮询，未修改 `history`。

## 命令行

- 无状态：每次从 `--input` 读取文件或目录。
- 只接受 `version: 2` 的记录导出；目录中的存储备份会跳过。
- 输入按记录 `id` 去重，批次和会话按 `id` 合并。
- 判分和统计在内存中完成，不写缓存或数据库。

## 测试范围

- Rust 单元测试：合格线、得点区分、试卷编号、题型顺序。
- Rust 集成测试：无状态读取、判分、错题、网站计数、连续答题时间、目录输入。
- 用户脚本：在 Chromium 中注入并检查单题保存、整套抓取、连续答题、收藏集、存储面板和导出。

页面结构或网站数据格式变化后，优先检查 `extension/jalaplus.user.js` 中的路由解析、题目数据提取和选择读取逻辑。
