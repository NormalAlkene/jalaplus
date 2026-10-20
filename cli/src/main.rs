mod model;
mod report;
mod score;
mod store;

use anyhow::{bail, Result};
use clap::{ArgAction, Parser, Subcommand};
use model::Record;
use score::PaperSummary;
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
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
    /// 一个或多个导出文件或目录，可重复
    #[arg(long = "input", global = true, action = ArgAction::Append, value_name = "文件或目录")]
    inputs: Vec<PathBuf>,

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
    /// 查看各试卷的作答数量、正确率和最近记录
    Overview,
    /// 查看一套试卷的估算分数、题型正确率和错题
    Paper {
        /// 试卷，例如 n2/2020-12
        paper: String,
        /// JSON 输出时不包含错题详情
        #[arg(long)]
        no_wrong: bool,
        /// 同时查看网站累计计数
        #[arg(long)]
        site_stats: bool,
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
        /// 使用网站累计计数，而不是记录作答
        #[arg(long)]
        site_stats: bool,
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
        /// 使用网站累计计数，而不是记录作答
        #[arg(long)]
        site_stats: bool,
    },
    /// 比较两套或多套试卷
    Compare {
        /// 试卷列表，例如 n2/2020-07 n2/2020-12；不填时比较全部
        papers: Vec<String>,
    },
    /// 查看连续答题的页面用时和会话用时
    Timing {
        /// 只查看一套试卷，例如 n2/2020-12
        paper: Option<String>,
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
    let dataset = store::load_inputs(&cli.inputs)?;
    let output = Output {
        json: cli.json,
        pretty: cli.pretty,
    };
    match cli.command {
        Command::Overview => {
            let keys = store::paper_keys(&dataset.records);
            let mut summaries = Vec::new();
            for key in keys.iter().rev() {
                let records = paper_records(&dataset.records, key);
                if !records.is_empty() {
                    summaries.push(score::summarize(
                        &records,
                        dataset.batch_for(key),
                        false,
                        false,
                        false,
                    ));
                }
            }
            let questions: BTreeSet<String> =
                dataset.records.iter().map(Record::question_key).collect();
            let totals = json!({
                "papers": summaries.len(),
                "records": dataset.records.len(),
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
        Command::Paper {
            paper,
            no_wrong,
            site_stats,
        } => {
            let key = store::normalize_paper(&paper)?;
            let records = paper_records(&dataset.records, &key);
            if records.is_empty() {
                bail!("没有找到试卷 {key} 的记录");
            }
            let summary = score::summarize(
                &records,
                dataset.batch_for(&key),
                true,
                !no_wrong,
                site_stats,
            );
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
            site_stats,
        } => {
            if site_stats {
                let mut items = site_wrong_items(&dataset, paper.as_deref(), kind.as_deref())?;
                let total = items.len();
                if !all {
                    items.truncate(limit);
                }
                if output.is_json() {
                    output.emit(&json!({
                        "count": total,
                        "shown": items.len(),
                        "items": items,
                    }))?;
                } else {
                    print_site_wrong_items(&items, total);
                }
                return Ok(());
            }

            let mut records: Vec<&Record> = match paper.as_deref() {
                Some(paper) => {
                    let key = store::normalize_paper(paper)?;
                    paper_records(&dataset.records, &key)
                }
                None => dataset.records.iter().collect(),
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
                let summary =
                    score::summarize(&subset, dataset.batch_for(&key), false, true, false);
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
            let mut records: Vec<&Record> = if let Some(reference) = parse_reference(&target) {
                dataset
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
                dataset
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
        Command::Stats {
            paper,
            by,
            site_stats,
        } => {
            if !matches!(by.as_str(), "chapter" | "category" | "paper") {
                bail!("--by 只能是 chapter、category 或 paper");
            }
            let mut keys = store::paper_keys(&dataset.records);
            if let Some(paper) = paper.as_deref() {
                let key = store::normalize_paper(paper)?;
                keys.retain(|item| item == &key);
                if keys.is_empty() {
                    bail!("没有找到试卷 {key} 的记录");
                }
            }
            let mut rows: Vec<StatRow> = Vec::new();
            for key in &keys {
                let records = paper_records(&dataset.records, key);
                if site_stats {
                    rows.extend(site_stat_rows(&records, &by));
                    continue;
                }
                let summary =
                    score::summarize(&records, dataset.batch_for(key), true, false, false);
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
            if by == "category" || by == "chapter" || (site_stats && by == "paper") {
                rows = merge_stat_rows(rows, &by);
            }
            if output.is_json() {
                output.emit(&json!({ "by": by, "siteStats": site_stats, "rows": rows }))?;
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
            let mut keys: Vec<String> = if papers.is_empty() {
                store::paper_keys(&dataset.records)
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
                let records = paper_records(&dataset.records, &key);
                if records.is_empty() {
                    bail!("没有找到试卷 {key} 的记录");
                }
                summaries.push(score::summarize(
                    &records,
                    dataset.batch_for(&key),
                    true,
                    false,
                    false,
                ));
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
        Command::Timing { paper } => {
            let mut records: Vec<&Record> = dataset
                .records
                .iter()
                .filter(|record| record.timing.is_some())
                .collect();
            if let Some(paper) = paper.as_deref() {
                let key = store::normalize_paper(paper)?;
                records.retain(|record| record.paper_key() == key);
            }
            if records.is_empty() {
                bail!("没有连续答题时间记录");
            }
            let mut rows: Vec<TimingRow> = records
                .iter()
                .filter_map(|record| {
                    let timing = record.timing.as_ref()?;
                    let (answered, correct) = attempt_counts(record);
                    Some(TimingRow {
                        record_id: record.id.clone(),
                        paper: record.paper_key(),
                        question_ref: record.question_key(),
                        session_id: timing.session_id.clone(),
                        answered_at: timing.answered_at.clone(),
                        page_ms: timing.page_ms,
                        session_ms: timing.session_ms,
                        navigation: timing.navigation.clone(),
                        answered,
                        correct,
                    })
                })
                .collect();
            rows.sort_by(|a, b| a.answered_at.cmp(&b.answered_at));
            let page_values: Vec<u64> = rows.iter().map(|row| row.page_ms).collect();
            let session_values: Vec<u64> = rows.iter().map(|row| row.session_ms).collect();
            let session_ids: BTreeSet<String> =
                rows.iter().map(|row| row.session_id.clone()).collect();
            let sessions = dataset
                .sessions
                .iter()
                .filter(|session| session_ids.contains(&session.id))
                .count();
            let summary = TimingSummary {
                count: rows.len(),
                sessions,
                page_ms: TimingStats::from(&page_values),
                session_ms: TimingStats::from(&session_values),
                rows,
            };
            if output.is_json() {
                output.emit(&summary)?;
            } else {
                print_timing_summary(&summary);
            }
        }
    }
    Ok(())
}

fn print_timing_summary(summary: &TimingSummary) {
    println!(
        "连续答题：{} 条，{} 个会话",
        summary.count, summary.sessions
    );
    println!(
        "页面用时：平均 {}，中位数 {}，最短 {}，最长 {}",
        format_duration(summary.page_ms.mean),
        format_duration(summary.page_ms.median),
        format_duration(summary.page_ms.min),
        format_duration(summary.page_ms.max)
    );
    println!(
        "会话用时：平均 {}，中位数 {}，最短 {}，最长 {}",
        format_duration(summary.session_ms.mean),
        format_duration(summary.session_ms.median),
        format_duration(summary.session_ms.min),
        format_duration(summary.session_ms.max)
    );
    println!();
    println!(
        "{:<10} {:<32} {:>10} {:>10} {:>6}",
        "试卷", "题目", "页面", "会话", "正确"
    );
    for row in &summary.rows {
        println!(
            "{:<10} {:<32} {:>10} {:>10} {:>6}",
            row.paper,
            row.question_ref,
            format_duration(row.page_ms),
            format_duration(row.session_ms),
            if row.answered == 0 {
                "-".to_string()
            } else {
                format!("{}/{}", row.correct, row.answered)
            }
        );
    }
}

fn print_site_wrong_items(items: &[SiteWrongItem], total: usize) {
    println!("网站计数错题：{total} 个页面");
    for item in items {
        println!();
        println!(
            "{} · {} / {} · 第{}题",
            item.paper, item.category_label, item.chapter_label, item.index
        );
        println!(
            "正确 {} 次，错误 {} 次，正确率 {}",
            item.correct,
            item.wrong,
            report::pct(item.accuracy)
        );
        if !item.prompt.is_empty() {
            println!("{}", item.prompt);
        }
    }
}

fn format_duration(ms: u64) -> String {
    let seconds = (ms as f64 / 1000.0).round() as u64;
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m{:02}s", seconds / 60, seconds % 60)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimingRow {
    record_id: String,
    paper: String,
    question_ref: String,
    session_id: String,
    answered_at: String,
    page_ms: u64,
    session_ms: u64,
    navigation: String,
    answered: usize,
    correct: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimingSummary {
    count: usize,
    sessions: usize,
    page_ms: TimingStats,
    session_ms: TimingStats,
    rows: Vec<TimingRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimingStats {
    mean: u64,
    median: u64,
    min: u64,
    max: u64,
}

impl TimingStats {
    fn from(values: &[u64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        let len = sorted.len();
        if len == 0 {
            return Self {
                mean: 0,
                median: 0,
                min: 0,
                max: 0,
            };
        }
        Self {
            mean: (sorted.iter().sum::<u64>() as f64 / len as f64).round() as u64,
            median: if len.is_multiple_of(2) {
                (sorted[len / 2 - 1] + sorted[len / 2]) / 2
            } else {
                sorted[len / 2]
            },
            min: sorted[0],
            max: sorted[len - 1],
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SiteWrongItem {
    paper: String,
    question_ref: String,
    category: String,
    chapter: String,
    category_label: String,
    chapter_label: String,
    index: u32,
    groups: usize,
    correct: u32,
    wrong: u32,
    accuracy: Option<f64>,
    prompt: String,
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

fn site_wrong_items(
    dataset: &store::Dataset,
    paper: Option<&str>,
    kind: Option<&str>,
) -> Result<Vec<SiteWrongItem>> {
    let paper_key = match paper {
        Some(paper) => Some(store::normalize_paper(paper)?),
        None => None,
    };
    let mut latest: BTreeMap<String, &Record> = BTreeMap::new();
    for record in &dataset.records {
        if paper_key
            .as_deref()
            .is_some_and(|key| record.paper_key() != key)
        {
            continue;
        }
        if let Some(kind) = kind {
            if !matches_kind(record, kind) {
                continue;
            }
        }
        if record.question.site_stats.is_none() {
            continue;
        }
        let key = record.question_key();
        let replace = latest
            .get(&key)
            .is_none_or(|current| record.captured_at > current.captured_at);
        if replace {
            latest.insert(key, record);
        }
    }
    let mut items: Vec<SiteWrongItem> = latest
        .into_values()
        .filter_map(|record| {
            let stats = record.question.site_stats.as_ref()?;
            if stats.wrong == 0 {
                return None;
            }
            Some(SiteWrongItem {
                paper: record.paper_key(),
                question_ref: record.question_key(),
                category: record.question.category.clone(),
                chapter: record.question.chapter.clone(),
                category_label: record.category_label().to_string(),
                chapter_label: record.chapter_label().to_string(),
                index: record.question.index,
                groups: record.group_count(),
                correct: stats.correct,
                wrong: stats.wrong,
                accuracy: ratio(
                    stats.correct as usize,
                    (stats.correct + stats.wrong) as usize,
                ),
                prompt: record.question.prompt.clone(),
            })
        })
        .collect();
    items.sort_by(|a, b| {
        a.paper
            .cmp(&b.paper)
            .then_with(|| {
                store::category_order(&a.category).cmp(&store::category_order(&b.category))
            })
            .then_with(|| a.chapter.cmp(&b.chapter))
            .then_with(|| a.index.cmp(&b.index))
    });
    Ok(items)
}

fn site_stat_rows(records: &[&Record], by: &str) -> Vec<StatRow> {
    let mut latest: BTreeMap<String, &Record> = BTreeMap::new();
    for record in records {
        if record.question.site_stats.is_none() {
            continue;
        }
        let key = record.question_key();
        let replace = latest
            .get(&key)
            .is_none_or(|current| record.captured_at > current.captured_at);
        if replace {
            latest.insert(key, record);
        }
    }
    let mut rows: Vec<StatRow> = Vec::new();
    for record in latest.values() {
        let stats = record.question.site_stats.as_ref().unwrap();
        let attempts = (stats.correct + stats.wrong) as usize;
        let (key, label) = match by {
            "paper" => (record.paper_key(), record.display_title()),
            "category" => (
                record.question.category.clone(),
                record.category_label().to_string(),
            ),
            _ => (
                format!("{}/{}", record.question.category, record.question.chapter),
                format!("{} · {}", record.category_label(), record.chapter_label()),
            ),
        };
        rows.push(StatRow {
            key,
            label,
            expected: record.group_count(),
            answered: attempts,
            correct: stats.correct as usize,
            accuracy: ratio(stats.correct as usize, attempts),
        });
    }
    rows
}

fn attempt_counts(record: &Record) -> (usize, usize) {
    let Some(answers) = record
        .attempt
        .as_ref()
        .and_then(|attempt| attempt.answers.as_ref())
    else {
        return (0, 0);
    };
    let mut answered = 0;
    let mut correct = 0;
    for group in &record.question.groups {
        let Some(user) = answers.get(&group.id) else {
            continue;
        };
        answered += 1;
        if group.answer.as_deref() == Some(user.as_str()) {
            correct += 1;
        }
    }
    (answered, correct)
}

fn ratio(correct: usize, total: usize) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some((((correct as f64 / total as f64) * 10000.0).round()) / 10000.0)
    }
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
        row.accuracy = ratio(row.correct, row.answered);
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
