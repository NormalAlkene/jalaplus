use crate::model::Record;
use crate::score::{PaperSummary, WrongItem};
use std::collections::BTreeMap;

pub fn pct(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{:.1}%", value * 100.0),
        None => "-".to_string(),
    }
}

pub fn short_time(value: &str) -> String {
    if value.len() >= 16 && value.as_bytes().get(10) == Some(&b'T') {
        format!("{} {}", &value[..10], &value[11..16])
    } else {
        value.to_string()
    }
}

pub fn complete_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "完整",
        Some(false) => "不完整",
        None => "未知",
    }
}

pub fn passed_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "合格",
        Some(false) => "未合格",
        None => "无法判断",
    }
}

pub fn print_overview(summaries: &[PaperSummary]) {
    if summaries.is_empty() {
        println!("还没有记录，请通过 --input 提供导出文件。");
        return;
    }
    let headers = [
        "试卷",
        "页面",
        "小题",
        "已答",
        "正确",
        "错误",
        "正确率",
        "完整",
        "最近记录",
    ];
    let rows: Vec<Vec<String>> = summaries
        .iter()
        .map(|summary| {
            vec![
                summary.paper.clone(),
                match summary.page_count {
                    Some(count) => format!("{}/{}", summary.captured_pages, count),
                    None => summary.captured_pages.to_string(),
                },
                summary.expected_groups.to_string(),
                summary.answered_groups.to_string(),
                summary.correct_groups.to_string(),
                summary.wrong_groups.to_string(),
                pct(summary.accuracy),
                complete_label(summary.complete).to_string(),
                summary
                    .last_captured_at
                    .as_deref()
                    .map(short_time)
                    .unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect();
    print_table(&headers, &rows);
}

pub fn print_paper(summary: &PaperSummary) {
    println!("试卷：{}", summary.title);
    let coverage = if summary.expected_groups == 0 {
        0.0
    } else {
        summary.answered_groups as f64 / summary.expected_groups as f64
    };
    let pages = match summary.page_count {
        Some(count) => format!("{} / {} 个页面", summary.captured_pages, count),
        None => format!("{} 个页面", summary.captured_pages),
    };
    println!(
        "覆盖：{} / {} 小题（{}），{}",
        summary.answered_groups,
        summary.expected_groups,
        pct(Some(coverage)),
        pages
    );
    match summary.complete {
        Some(true) => {}
        Some(false) => println!("注意：抓取不完整，分数只覆盖已记录的小题。"),
        None => println!("注意：没有整套抓取信息，分数只覆盖已记录的小题。"),
    }
    match summary.total_score {
        Some(score) => println!(
            "估算总分：{} / {}（合格线 {}，{}）",
            score,
            summary.max_score,
            summary.pass_line,
            passed_label(summary.passed)
        ),
        None => println!("估算总分：无法判分（还没有作答记录）"),
    }
    println!();
    println!("得点区分：");
    for section in &summary.sections {
        println!(
            "  {}：{} / {}（基准点 {}；已答 {}/{}，正确 {}，正确率 {}）",
            section.label,
            section.score,
            section.max,
            section.min,
            section.answered,
            section.expected,
            section.correct,
            pct(section.accuracy)
        );
    }
    if let Some(stats) = summary.site_stats.as_ref() {
        println!();
        println!(
            "网站计数：{} 个页面，{} 个小题，正确 {}，错误 {}，正确率 {}",
            stats.questions,
            stats.groups,
            stats.correct,
            stats.wrong,
            pct(stats.accuracy)
        );
    }
    if !summary.chapters.is_empty() {
        println!();
        println!("题型：");
        let headers = ["题型", "小题", "已答", "正确", "正确率"];
        let rows: Vec<Vec<String>> = summary
            .chapters
            .iter()
            .filter(|chapter| chapter.expected > 0)
            .map(|chapter| {
                vec![
                    chapter.label.clone(),
                    chapter.expected.to_string(),
                    chapter.answered.to_string(),
                    chapter.correct.to_string(),
                    pct(chapter.accuracy),
                ]
            })
            .collect();
        print_table(&headers, &rows);
    }
    if !summary.wrong.is_empty() {
        println!();
        println!(
            "错题 {} 道。运行 jalaquery wrong {} 查看用户答案和解析。",
            summary.wrong.len(),
            summary.paper
        );
    }
}

pub fn print_wrong(items: &[WrongItem], total: usize, show_explanation: bool) {
    if items.is_empty() {
        println!("没有错题。");
        return;
    }
    println!("错题：{} 道（显示 {} 道）", total, items.len());
    for (index, item) in items.iter().enumerate() {
        println!();
        println!(
            "{}. {} · {} / {} · 第{}题 · {}",
            index + 1,
            item.paper,
            item.category_label,
            item.chapter_label,
            item.index,
            item.group
        );
        println!("   题干：{}", item.prompt.replace('\n', " "));
        println!("   你选：{} {}", item.user.number, item.user.label);
        println!("   正确：{} {}", item.answer.number, item.answer.label);
        if show_explanation {
            match &item.explanation {
                Some(explanation) if !explanation.trim().is_empty() => {
                    println!("   解析：");
                    for line in explanation.lines() {
                        println!("     {line}");
                    }
                }
                _ => println!("   解析：没有可用的解析"),
            }
        }
    }
}

pub fn print_stats(rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("没有统计数据。");
        return;
    }
    let headers = ["范围", "小题", "已答", "正确", "正确率"];
    print_table(&headers, rows);
}

pub fn print_compare(summaries: &[PaperSummary], chapters: &[CompareChapter]) {
    if summaries.is_empty() {
        println!("没有可比较的试卷。");
        return;
    }
    let mut section_labels: Vec<String> = Vec::new();
    for summary in summaries {
        for section in &summary.sections {
            if !section_labels.contains(&section.label) {
                section_labels.push(section.label.clone());
            }
        }
    }
    let mut headers = vec!["试卷".to_string(), "已答".to_string(), "正确率".to_string()];
    headers.extend(section_labels.iter().cloned());
    let mut rows: Vec<Vec<String>> = Vec::new();
    for summary in summaries {
        let mut row = vec![
            summary.paper.clone(),
            summary.answered_groups.to_string(),
            pct(summary.accuracy),
        ];
        for label in &section_labels {
            let accuracy = summary
                .sections
                .iter()
                .find(|section| &section.label == label)
                .and_then(|section| section.accuracy);
            row.push(pct(accuracy));
        }
        rows.push(row);
    }
    print_table_refs(&headers, &rows);

    if !chapters.is_empty() {
        println!();
        println!("相同题型：");
        let mut headers = vec!["题型".to_string()];
        headers.extend(summaries.iter().map(|summary| summary.paper.clone()));
        let rows: Vec<Vec<String>> = chapters
            .iter()
            .map(|chapter| {
                let mut row = vec![chapter.label.clone()];
                for summary in summaries {
                    let cell = chapter
                        .cells
                        .get(&summary.paper)
                        .map(|(correct, answered)| {
                            if *answered == 0 {
                                "-".to_string()
                            } else {
                                format!("{}/{}", correct, answered)
                            }
                        })
                        .unwrap_or_else(|| "-".to_string());
                    row.push(cell);
                }
                row
            })
            .collect();
        print_table_refs(&headers, &rows);
    }
}

#[derive(Debug, Clone)]
pub struct CompareChapter {
    pub key: String,
    pub label: String,
    pub cells: BTreeMap<String, (usize, usize)>,
}

pub fn build_compare_chapters(summaries: &[PaperSummary]) -> Vec<CompareChapter> {
    let mut chapters: BTreeMap<String, CompareChapter> = BTreeMap::new();
    for summary in summaries {
        for chapter in &summary.chapters {
            let entry = chapters
                .entry(chapter.key.clone())
                .or_insert_with(|| CompareChapter {
                    key: chapter.key.clone(),
                    label: chapter.label.clone(),
                    cells: BTreeMap::new(),
                });
            entry
                .cells
                .insert(summary.paper.clone(), (chapter.correct, chapter.answered));
        }
    }
    let mut rows: Vec<CompareChapter> = chapters.into_values().collect();
    rows.sort_by(|a, b| {
        let left = a.key.split('/').next().unwrap_or("");
        let right = b.key.split('/').next().unwrap_or("");
        let left_chapter = a.key.split('/').nth(1).unwrap_or("");
        let right_chapter = b.key.split('/').nth(1).unwrap_or("");
        crate::store::category_order(left)
            .cmp(&crate::store::category_order(right))
            .then_with(|| {
                crate::store::chapter_order(left, left_chapter)
                    .cmp(&crate::store::chapter_order(right, right_chapter))
            })
            .then_with(|| a.key.cmp(&b.key))
    });
    rows
}

pub fn print_record(record: &Record, show_explanation: bool) {
    println!(
        "记录：{} · {} · {}",
        record.id,
        short_time(&record.captured_at),
        match record.kind {
            crate::model::Kind::Paper => "整套抓取",
            crate::model::Kind::Question => "单题保存",
            crate::model::Kind::Answer => "连续作答",
        }
    );
    if let Some(batch) = &record.batch_id {
        println!("抓取批次：{batch}");
    }
    println!("试卷：{}（{}）", record.display_title(), record.paper_key());
    println!(
        "题目：{} / {} · 第{}题",
        record.category_label(),
        record.chapter_label(),
        record.question.index
    );
    if let Some(url) = &record.question.url {
        println!("地址：{url}");
    }
    if !record.question.prompt.is_empty() {
        println!();
        println!("题干：");
        println!("{}", record.question.prompt);
    }
    if let Some(passage) = &record.question.passage {
        println!();
        println!("文章：");
        println!("{}", passage.text);
        if let Some(translation) = &passage.translation {
            println!();
            println!("翻译：");
            println!("{translation}");
        }
    }
    let answers = record
        .attempt
        .as_ref()
        .and_then(|attempt| attempt.answers.as_ref());
    for (index, group) in record.question.groups.iter().enumerate() {
        println!();
        println!(
            "{}小题 {}{}",
            if record.question.groups.len() > 1 {
                format!("第{}", index + 1)
            } else {
                String::new()
            },
            group.id,
            if group.prompt.trim().is_empty() {
                String::new()
            } else {
                format!("：{}", group.prompt)
            }
        );
        for (option_index, option) in group.options.iter().enumerate() {
            let mut marks = Vec::new();
            if answers
                .and_then(|map| map.get(&group.id))
                .map(String::as_str)
                == Some(option.id.as_str())
            {
                marks.push("你的答案");
            }
            if group.answer.as_deref() == Some(option.id.as_str()) {
                marks.push("正确答案");
            }
            let mark = if marks.is_empty() {
                String::new()
            } else {
                format!("  ← {}", marks.join("，"))
            };
            println!("  {}. {}{}", option_index + 1, option.label, mark);
        }
        if let Some(user) = answers.and_then(|map| map.get(&group.id)) {
            let value = record.answer_value(group, user);
            println!("  你的答案：{} {}", value.number, value.label);
        } else {
            println!("  你的答案：未作答");
        }
        if let Some(answer) = &group.answer {
            let value = record.answer_value(group, answer);
            println!("  正确答案：{} {}", value.number, value.label);
        } else {
            println!("  正确答案：未提供");
        }
        if show_explanation {
            let explanation = group
                .explanation
                .as_deref()
                .or(record.question.explanation.as_deref());
            match explanation {
                Some(text) if !text.trim().is_empty() => {
                    println!("  解析：");
                    for line in text.lines() {
                        println!("    {line}");
                    }
                }
                _ => {
                    let status = if record.access.explanation == "locked" {
                        "未解锁（需要登录）"
                    } else {
                        "没有可用的解析"
                    };
                    println!("  解析：{status}");
                }
            }
        }
    }
    if let Some(transcript) = &record.question.transcript {
        println!();
        println!("听力原文：");
        let roles: BTreeMap<i32, &str> = transcript
            .roles
            .iter()
            .map(|role| (role.role_id, role.name.as_str()))
            .collect();
        for sentence in &transcript.sentences {
            let role = roles.get(&sentence.role_id).copied().unwrap_or("");
            let role = if role.is_empty() {
                String::new()
            } else {
                format!("{role}：")
            };
            println!(
                "  {} {}{}",
                format_ms(sentence.start_ms),
                role,
                sentence.text
            );
        }
    } else if record.access.transcript == "locked" {
        println!();
        println!("听力原文：未解锁（需要登录）");
    }
    if let Some(stats) = record.question.site_stats.as_ref() {
        println!();
        println!("网站计数：正确 {}，错误 {}", stats.correct, stats.wrong);
    }
    if let Some(timing) = record.timing.as_ref() {
        println!();
        println!(
            "用时：页面 {} ms，会话 {} ms（{}）",
            timing.page_ms, timing.session_ms, timing.navigation
        );
    }
}

fn format_ms(ms: u64) -> String {
    let total = ms / 1000;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let headers: Vec<String> = headers.iter().map(|header| (*header).to_string()).collect();
    print_table_refs(&headers, rows);
}

fn print_table_refs(headers: &[String], rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = headers.iter().map(|header| display_width(header)).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(display_width(cell));
            }
        }
    }
    print_row(headers, &widths);
    let separator: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    print_row(&separator, &widths);
    for row in rows {
        print_row(row, &widths);
    }
}

fn print_row(cells: &[String], widths: &[usize]) {
    let mut line = String::new();
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            line.push_str("  ");
        }
        line.push_str(cell);
        if let Some(width) = widths.get(index) {
            let padding = width.saturating_sub(display_width(cell));
            line.push_str(&" ".repeat(padding));
        }
    }
    println!("{}", line.trim_end());
}

fn display_width(value: &str) -> usize {
    value.chars().map(char_width).sum()
}

fn char_width(value: char) -> usize {
    let code = value as u32;
    if matches!(
        code,
        0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}
