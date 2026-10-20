mod model;
mod report;
mod score;
mod store;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use model::Record;
use score::PaperSummary;
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "jalaquery",
    version,
    about = "查询和判分 JaLaPlus 导出的 JLPT 作答记录",
    long_about = None,
    disable_help_subcommand = true,
    help_template = "{about-with-newline}\n用法: {usage}\n\n命令:\n{subcommands}\n\n选项:\n{options}"
)]
struct Cli {
    /// 数据文件路径，默认 $XDG_DATA_HOME/jalaquery/store.json
    #[arg(long, global = true, value_name = "文件")]
    store: Option<PathBuf>,

    /// 输出紧凑 JSON
    #[arg(long, global = true)]
    json: bool,

    /// 输出带缩进的 JSON
    #[arg(long, global = true)]
    pretty: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 导入用户脚本导出的 JSON 文件
    Import {
        /// 一个或多个 JSON 文件，也可以传入目录
        #[arg(required = true, value_name = "文件")]
        files: Vec<PathBuf>,
    },
    /// 查看各试卷的作答数量、正确率和最近记录
    Overview,
    /// 查看一套试卷的估算分数、题型正确率和错题
    Paper {
        /// 试卷，例如 n2/2020-12
        paper: String,
        /// JSON 输出时不包含错题详情
        #[arg(long)]
        no_wrong: bool,
    },
    /// 查看错题、用户答案、正确答案和解析
    Wrong {
        /// 只查看一套试卷，例如 n2/2020-12
        paper: Option<String>,
        /// 只查看一个题型，例如 word-formation、词汇/构词 或 构词
        #[arg(long = "type", value_name = "题型")]
        kind: Option<String>,
        /// 最多显示多少道错题
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// 显示全部错题
        #[arg(long)]
        all: bool,
        /// 不输出解析
        #[arg(long)]
        no_explanation: bool,
    },
    /// 查看某道题的全部记录，或按记录编号查看
    Show {
        /// 题目引用（n2/2020-12/vocabulary/word-formation/1）或记录编号
        target: String,
        /// 不输出解析
        #[arg(long)]
        no_explanation: bool,
    },
    /// 按题型、类别或试卷统计正确率
    Stats {
        /// 只统计一套试卷
        paper: Option<String>,
        /// 统计维度：chapter、category、paper
        #[arg(long, default_value = "chapter")]
        by: String,
    },
    /// 比较两套或多套试卷
    Compare {
        /// 试卷列表，例如 n2/2020-07 n2/2020-12；不填时比较全部
        papers: Vec<String>,
    },
    /// 查看数据文件信息
    Store,
    /// 删除数据文件
    Clear {
        /// 确认删除
        #[arg(long)]
        yes: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("错误：{error:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let path = store::resolve_store_path(cli.store.clone())?;
    let output = Output {
        json: cli.json,
        pretty: cli.pretty,
    };
    match cli.command {
        Command::Import { files } => {
            let stats = store::import_paths(&files, &path)?;
            if output.is_json() {
                output.emit(&json!({
                    "added": stats.added,
                    "updated": stats.updated,
                    "skipped": stats.skipped,
                    "files": stats.files,
                    "store": path.display().to_string(),
                }))?;
            } else {
                report::print_import(stats.added, stats.updated, stats.skipped, stats.files);
            }
        }
        Command::Overview => {
            let store = store::load(&path)?;
            let keys = store::paper_keys(&store.records);
            let mut summaries = Vec::new();
            for key in keys.iter().rev() {
                let records = paper_records(&store.records, key);
                if !records.is_empty() {
                    summaries.push(score::summarize(&records, false, false));
                }
            }
            let questions: BTreeSet<String> =
                store.records.iter().map(Record::question_key).collect();
            let totals = json!({
                "papers": summaries.len(),
                "records": store.records.len(),
                "questions": questions.len(),
                "answered_groups": summaries.iter().map(|s| s.answered_groups).sum::<usize>(),
                "correct_groups": summaries.iter().map(|s| s.correct_groups).sum::<usize>(),
            });
            if output.is_json() {
                output.emit(&json!({ "papers": summaries, "totals": totals }))?;
            } else {
                report::print_overview(&summaries);
            }
        }
        Command::Paper { paper, no_wrong } => {
            let key = store::normalize_paper(&paper)?;
            let store = store::load(&path)?;
            let records = paper_records(&store.records, &key);
            if records.is_empty() {
                bail!("没有找到试卷 {key} 的记录");
            }
            let summary = score::summarize(&records, true, !no_wrong);
            if output.is_json() {
                output.emit(&summary)?;
            } else {
                report::print_paper(&summary);
            }
        }
        Command::Wrong {
            paper,
            kind,
            limit,
            all,
            no_explanation,
        } => {
            let store = store::load(&path)?;
            let mut records: Vec<&Record> = match paper.as_deref() {
                Some(paper) => {
                    let key = store::normalize_paper(paper)?;
                    paper_records(&store.records, &key)
                }
                None => store.records.iter().collect(),
            };
            if records.is_empty() {
                bail!("没有可查询的记录");
            }
            if let Some(kind) = kind.as_deref() {
                records.retain(|record| matches_kind(record, kind));
            }
            let mut keys: Vec<String> = records.iter().map(|record| record.paper_key()).collect();
            keys.sort();
            keys.dedup();
            let mut items = Vec::new();
            for key in keys {
                let subset: Vec<&Record> = records
                    .iter()
                    .copied()
                    .filter(|record| record.paper_key() == key)
                    .collect();
                let summary = score::summarize(&subset, false, true);
                items.extend(summary.wrong);
            }
            let total = items.len();
            if !all {
                items.truncate(limit);
            }
            if no_explanation {
                for item in &mut items {
                    item.explanation = None;
                }
            }
            if output.is_json() {
                output.emit(&json!({
                    "count": total,
                    "shown": items.len(),
                    "items": items,
                }))?;
            } else {
                report::print_wrong(&items, total, !no_explanation);
            }
        }
        Command::Show {
            target,
            no_explanation,
        } => {
            let store = store::load(&path)?;
            let mut records: Vec<&Record> = if let Some(reference) = parse_reference(&target) {
                store
                    .records
                    .iter()
                    .filter(|record| {
                        record.paper.level == reference.0
                            && record.paper.exam == reference.1
                            && record.question.category == reference.2
                            && record.question.chapter == reference.3
                            && record.question.index == reference.4
                    })
                    .collect()
            } else {
                store
                    .records
                    .iter()
                    .filter(|record| record.id == target)
                    .collect()
            };
            if records.is_empty() {
                bail!("没有找到记录：{target}");
            }
            records.sort_by(|a, b| {
                b.captured_at
                    .cmp(&a.captured_at)
                    .then_with(|| b.id.cmp(&a.id))
            });
            if output.is_json() {
                output.emit(&json!({ "records": records }))?;
            } else {
                for (index, record) in records.iter().enumerate() {
                    if index > 0 {
                        println!();
                        println!("{}", "-".repeat(60));
                    }
                    report::print_record(record, !no_explanation);
                }
            }
        }
        Command::Stats { paper, by } => {
            if !matches!(by.as_str(), "chapter" | "category" | "paper") {
                bail!("--by 只能是 chapter、category 或 paper");
            }
            let store = store::load(&path)?;
            let mut keys = store::paper_keys(&store.records);
            if let Some(paper) = paper.as_deref() {
                let key = store::normalize_paper(paper)?;
                keys.retain(|item| item == &key);
                if keys.is_empty() {
                    bail!("没有找到试卷 {key} 的记录");
                }
            }
            let mut rows: Vec<StatRow> = Vec::new();
            for key in &keys {
                let records = paper_records(&store.records, key);
                let summary = score::summarize(&records, true, false);
                match by.as_str() {
                    "paper" => rows.push(StatRow {
                        key: summary.paper.clone(),
                        label: summary.title.clone(),
                        expected: summary.expected_groups,
                        answered: summary.answered_groups,
                        correct: summary.correct_groups,
                        accuracy: summary.accuracy,
                    }),
                    "category" => rows.extend(category_rows(&summary)),
                    _ => rows.extend(summary.chapters.iter().map(|chapter| StatRow {
                        key: chapter.key.clone(),
                        label: chapter.label.clone(),
                        expected: chapter.expected,
                        answered: chapter.answered,
                        correct: chapter.correct,
                        accuracy: chapter.accuracy,
                    })),
                }
            }
            if by == "category" || by == "chapter" {
                rows = merge_stat_rows(rows, &by);
            }
            if output.is_json() {
                output.emit(&json!({ "by": by, "rows": rows }))?;
            } else {
                let table_rows: Vec<Vec<String>> = rows
                    .iter()
                    .map(|row| {
                        vec![
                            row.label.clone(),
                            row.expected.to_string(),
                            row.answered.to_string(),
                            row.correct.to_string(),
                            report::pct(row.accuracy),
                        ]
                    })
                    .collect();
                report::print_stats(&table_rows);
            }
        }
        Command::Compare { papers } => {
            let store = store::load(&path)?;
            let mut keys: Vec<String> = if papers.is_empty() {
                store::paper_keys(&store.records)
            } else {
                let mut result = Vec::new();
                for paper in &papers {
                    result.push(store::normalize_paper(paper)?);
                }
                result.sort();
                result.dedup();
                result
            };
            if keys.len() < 2 {
                bail!("请至少提供两套试卷");
            }
            let mut summaries: Vec<PaperSummary> = Vec::new();
            for key in keys.drain(..) {
                let records = paper_records(&store.records, &key);
                if records.is_empty() {
                    bail!("没有找到试卷 {key} 的记录");
                }
                summaries.push(score::summarize(&records, true, false));
            }
            let chapters = report::build_compare_chapters(&summaries);
            if output.is_json() {
                let papers_json: Vec<_> = summaries
                    .iter()
                    .map(|summary| {
                        json!({
                            "paper": summary.paper,
                            "title": summary.title,
                            "answered_groups": summary.answered_groups,
                            "correct_groups": summary.correct_groups,
                            "accuracy": summary.accuracy,
                            "sections": summary.sections,
                        })
                    })
                    .collect();
                let chapters_json: Vec<_> = chapters
                    .iter()
                    .map(|chapter| {
                        let papers: BTreeMap<String, _> = chapter
                            .cells
                            .iter()
                            .map(|(paper, (correct, answered))| {
                                (
                                    paper.clone(),
                                    json!({ "correct": correct, "answered": answered }),
                                )
                            })
                            .collect();
                        json!({
                            "key": chapter.key,
                            "label": chapter.label,
                            "papers": papers,
                        })
                    })
                    .collect();
                output.emit(&json!({ "papers": papers_json, "chapters": chapters_json }))?;
            } else {
                report::print_compare(&summaries, &chapters);
            }
        }
        Command::Store => {
            let store = store::load(&path)?;
            let questions: BTreeSet<String> =
                store.records.iter().map(Record::question_key).collect();
            let papers = store::paper_keys(&store.records);
            if output.is_json() {
                output.emit(&json!({
                    "path": path.display().to_string(),
                    "size": store::store_size(&path),
                    "records": store.records.len(),
                    "papers": papers.len(),
                    "questions": questions.len(),
                }))?;
            } else {
                report::print_store(
                    &path.display().to_string(),
                    store::store_size(&path),
                    store.records.len(),
                    papers.len(),
                    questions.len(),
                );
            }
        }
        Command::Clear { yes } => {
            if !yes {
                bail!("删除数据文件需要 --yes");
            }
            if path.exists() {
                fs::remove_file(&path)
                    .with_context(|| format!("删除数据文件失败：{}", path.display()))?;
                println!("已删除 {}", path.display());
            } else {
                println!("数据文件不存在：{}", path.display());
            }
        }
    }
    Ok(())
}

struct Output {
    json: bool,
    pretty: bool,
}

impl Output {
    fn is_json(&self) -> bool {
        self.json || self.pretty
    }

    fn emit<T: Serialize>(&self, value: &T) -> Result<()> {
        let text = if self.pretty {
            serde_json::to_string_pretty(value)?
        } else {
            serde_json::to_string(value)?
        };
        println!("{text}");
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatRow {
    key: String,
    label: String,
    expected: usize,
    answered: usize,
    correct: usize,
    accuracy: Option<f64>,
}

fn paper_records<'a>(records: &'a [Record], key: &str) -> Vec<&'a Record> {
    records
        .iter()
        .filter(|record| record.paper_key() == key)
        .collect()
}

fn parse_reference(value: &str) -> Option<(String, String, String, String, u32)> {
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 5 {
        return None;
    }
    Some((
        parts[0].to_lowercase(),
        parts[1].to_string(),
        parts[2].to_string(),
        parts[3].to_string(),
        parts[4].parse().ok()?,
    ))
}

fn matches_kind(record: &Record, kind: &str) -> bool {
    let full = format!("{}/{}", record.question.category, record.question.chapter);
    kind == record.question.chapter
        || kind == record.chapter_label()
        || kind == full
        || kind == record.question.category
}

fn category_rows(summary: &PaperSummary) -> Vec<StatRow> {
    let mut rows = Vec::new();
    for chapter in &summary.chapters {
        let category = chapter.key.split('/').next().unwrap_or("");
        let label = chapter
            .label
            .split(" · ")
            .next()
            .unwrap_or(category)
            .to_string();
        rows.push(StatRow {
            key: category.to_string(),
            label,
            expected: chapter.expected,
            answered: chapter.answered,
            correct: chapter.correct,
            accuracy: chapter.accuracy,
        });
    }
    rows
}

fn merge_stat_rows(rows: Vec<StatRow>, by: &str) -> Vec<StatRow> {
    let mut merged: BTreeMap<String, StatRow> = BTreeMap::new();
    for row in rows {
        let entry = merged.entry(row.key.clone()).or_insert_with(|| StatRow {
            key: row.key.clone(),
            label: row.label.clone(),
            expected: 0,
            answered: 0,
            correct: 0,
            accuracy: None,
        });
        entry.expected += row.expected;
        entry.answered += row.answered;
        entry.correct += row.correct;
    }
    let mut rows: Vec<StatRow> = merged.into_values().collect();
    for row in &mut rows {
        row.accuracy = if row.answered == 0 {
            None
        } else {
            let value = row.correct as f64 / row.answered as f64;
            Some((value * 10000.0).round() / 10000.0)
        };
    }
    rows.sort_by(|a, b| {
        let left_category = a.key.split('/').next().unwrap_or("");
        let right_category = b.key.split('/').next().unwrap_or("");
        let left_chapter = a.key.split('/').nth(1).unwrap_or("");
        let right_chapter = b.key.split('/').nth(1).unwrap_or("");
        store::category_order(left_category)
            .cmp(&store::category_order(right_category))
            .then_with(|| {
                if by == "chapter" {
                    store::chapter_order(left_category, left_chapter)
                        .cmp(&store::chapter_order(right_category, right_chapter))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| a.key.cmp(&b.key))
    });
    rows
}
