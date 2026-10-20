use crate::model::{AnswerValue, Record};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperSummary {
    pub paper: String,
    pub title: String,
    pub level: String,
    pub exam: String,
    pub page_count: Option<usize>,
    pub captured_pages: usize,
    pub expected_groups: usize,
    pub answered_groups: usize,
    pub correct_groups: usize,
    pub wrong_groups: usize,
    pub accuracy: Option<f64>,
    pub complete: Option<bool>,
    pub last_captured_at: Option<String>,
    pub total_score: Option<u32>,
    pub max_score: u32,
    pub pass_line: u32,
    pub passed: Option<bool>,
    pub sections: Vec<SectionSummary>,
    pub chapters: Vec<ChapterSummary>,
    pub wrong: Vec<WrongItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionSummary {
    pub id: String,
    pub label: String,
    pub expected: usize,
    pub answered: usize,
    pub correct: usize,
    pub score: u32,
    pub max: u32,
    pub min: u32,
    pub points_per_group: f64,
    pub accuracy: Option<f64>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSummary {
    pub key: String,
    pub label: String,
    pub expected: usize,
    pub answered: usize,
    pub correct: usize,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongItem {
    pub record_id: String,
    pub paper: String,
    pub category: String,
    pub chapter: String,
    pub category_label: String,
    pub chapter_label: String,
    pub index: u32,
    pub group: String,
    pub prompt: String,
    pub user: AnswerValue,
    pub answer: AnswerValue,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct SectionAgg {
    expected: usize,
    answered: usize,
    correct: usize,
}

struct SectionMeta {
    id: &'static str,
    label: &'static str,
    max: u32,
    min: u32,
}

pub fn summarize(records: &[&Record], include_chapters: bool, include_wrong: bool) -> PaperSummary {
    let first = records[0];
    let level = first.paper.level.clone();
    let exam = first.paper.exam.clone();
    let paper = first.paper_key();
    let title = records
        .iter()
        .find(|record| record.paper.title.is_some())
        .and_then(|record| record.paper.title.clone())
        .unwrap_or_else(|| format!("JLPT {}・{}", level.to_uppercase(), exam));

    let mut latest_overall: BTreeMap<String, &Record> = BTreeMap::new();
    let mut latest_answered: BTreeMap<String, &Record> = BTreeMap::new();
    let mut max_groups: BTreeMap<String, usize> = BTreeMap::new();
    let mut chapter_of: BTreeMap<String, String> = BTreeMap::new();

    for record in records {
        let question_key = record.question_key();
        if newer(record, latest_overall.get(&question_key).copied()) {
            latest_overall.insert(question_key.clone(), record);
        }
        if record.has_answers() && newer(record, latest_answered.get(&question_key).copied()) {
            latest_answered.insert(question_key.clone(), record);
        }
        max_groups
            .entry(question_key.clone())
            .and_modify(|count| *count = (*count).max(record.group_count()))
            .or_insert_with(|| record.group_count());
        chapter_of
            .entry(question_key)
            .or_insert_with(|| format!("{}/{}", record.question.category, record.question.chapter));
    }

    let mut chapter_expected: BTreeMap<String, usize> = BTreeMap::new();
    for (question_key, count) in &max_groups {
        if let Some(chapter) = chapter_of.get(question_key) {
            *chapter_expected.entry(chapter.clone()).or_insert(0) += count;
        }
    }

    let latest_meta = records
        .iter()
        .filter(|record| record.paper.complete.is_some())
        .max_by(|a, b| compare_newer(a, b))
        .copied();
    if let Some(meta) = latest_meta.and_then(|record| record.paper.chapter_groups.as_ref()) {
        for (chapter, count) in meta {
            let expected = chapter_expected.entry(chapter.clone()).or_insert(0);
            *expected = (*expected).max(*count);
        }
    }

    let page_count = records
        .iter()
        .filter_map(|record| record.paper.page_count)
        .max();
    let captured_pages = max_groups.len();
    let mut complete = latest_meta.and_then(|record| record.paper.complete);
    if complete == Some(true) {
        if let Some(page_count) = page_count {
            if captured_pages < page_count {
                complete = Some(false);
            }
        }
    }

    let expected_groups: usize = chapter_expected.values().sum();
    let section_meta = section_meta(&level);
    let mut section_aggs: BTreeMap<&str, SectionAgg> = section_meta
        .iter()
        .map(|meta| {
            (
                meta.id,
                SectionAgg {
                    expected: 0,
                    answered: 0,
                    correct: 0,
                },
            )
        })
        .collect();

    for (chapter, count) in &chapter_expected {
        let category = chapter.split('/').next().unwrap_or("");
        let section = section_for(&level, category);
        if let Some(agg) = section_aggs.get_mut(section) {
            agg.expected += count;
        }
    }

    let mut chapter_answered: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut answer_values: Vec<(&Record, &crate::model::Group, &str)> = Vec::new();
    for question_key in max_groups.keys() {
        let record = latest_answered
            .get(question_key)
            .or_else(|| latest_overall.get(question_key));
        let Some(record) = record else { continue };
        let chapter = format!("{}/{}", record.question.category, record.question.chapter);
        let section = section_for(&level, &record.question.category);
        let Some(answers) = record.attempt.answers.as_ref() else {
            continue;
        };
        for group in &record.question.groups {
            let Some(user) = answers.get(&group.id) else {
                continue;
            };
            if let Some(agg) = section_aggs.get_mut(section) {
                agg.answered += 1;
                if group.answer.as_deref() == Some(user.as_str()) {
                    agg.correct += 1;
                }
            }
            let entry = chapter_answered.entry(chapter.clone()).or_insert((0, 0));
            entry.0 += 1;
            if group.answer.as_deref() == Some(user.as_str()) {
                entry.1 += 1;
            }
            if let Some(answer) = group.answer.as_deref() {
                if answer != user.as_str() {
                    answer_values.push((record, group, user));
                }
            }
        }
    }

    let mut sections = Vec::new();
    for meta in &section_meta {
        let agg = section_aggs.get(meta.id).copied().unwrap_or(SectionAgg {
            expected: 0,
            answered: 0,
            correct: 0,
        });
        let score = if agg.expected == 0 {
            0
        } else {
            ((agg.correct as f64 / agg.expected as f64) * meta.max as f64).round() as u32
        };
        let points_per_group = if agg.expected == 0 {
            0.0
        } else {
            round4(meta.max as f64 / agg.expected as f64)
        };
        sections.push(SectionSummary {
            id: meta.id.to_string(),
            label: meta.label.to_string(),
            expected: agg.expected,
            answered: agg.answered,
            correct: agg.correct,
            score,
            max: meta.max,
            min: meta.min,
            points_per_group,
            accuracy: ratio(agg.correct, agg.answered),
            passed: agg.expected > 0 && score >= meta.min,
        });
    }

    let last_captured_at = records
        .iter()
        .map(|record| record.captured_at.clone())
        .max();
    let answered_groups: usize = sections.iter().map(|section| section.answered).sum();
    let correct_groups: usize = sections.iter().map(|section| section.correct).sum();
    let wrong_groups = answered_groups.saturating_sub(correct_groups);
    let all_sections_passed = sections
        .iter()
        .filter(|section| section.expected > 0)
        .all(|section| section.passed);
    let total_score = if answered_groups > 0 && expected_groups > 0 {
        Some(sections.iter().map(|section| section.score).sum())
    } else {
        None
    };
    let pass_line = pass_line(&level);
    let complete_enough = matches!(complete, Some(true));
    let passed = match total_score {
        Some(total) if complete_enough => Some(total >= pass_line && all_sections_passed),
        _ => None,
    };

    let mut chapters = Vec::new();
    if include_chapters {
        let mut chapter_labels: BTreeMap<String, (String, String)> = BTreeMap::new();
        for record in records {
            let key = format!("{}/{}", record.question.category, record.question.chapter);
            chapter_labels.entry(key).or_insert_with(|| {
                (
                    record.category_label().to_string(),
                    record.chapter_label().to_string(),
                )
            });
        }
        let mut chapter_keys: Vec<&String> = chapter_expected.keys().collect();
        chapter_keys.sort_by(|a, b| {
            let left = a.split('/').next().unwrap_or("");
            let right = b.split('/').next().unwrap_or("");
            let left_chapter = a.split('/').nth(1).unwrap_or("");
            let right_chapter = b.split('/').nth(1).unwrap_or("");
            crate::store::category_order(left)
                .cmp(&crate::store::category_order(right))
                .then_with(|| {
                    crate::store::chapter_order(left, left_chapter)
                        .cmp(&crate::store::chapter_order(right, right_chapter))
                })
                .then_with(|| a.cmp(b))
        });
        for chapter in chapter_keys {
            let expected = chapter_expected.get(chapter).copied().unwrap_or(0);
            let (answered, correct) = chapter_answered.get(chapter).copied().unwrap_or((0, 0));
            let (category_label, chapter_label) = chapter_labels
                .get(chapter)
                .cloned()
                .unwrap_or_else(|| (chapter.clone(), String::new()));
            let label = if chapter_label.is_empty() {
                category_label
            } else {
                format!("{category_label} · {chapter_label}")
            };
            chapters.push(ChapterSummary {
                key: chapter.clone(),
                label,
                expected,
                answered,
                correct,
                accuracy: ratio(correct, answered),
            });
        }
    }

    let mut wrong = Vec::new();
    if include_wrong {
        answer_values.sort_by(|a, b| {
            let left = a.0;
            let right = b.0;
            left.paper_key()
                .cmp(&right.paper_key())
                .then_with(|| {
                    crate::store::category_order(&left.question.category)
                        .cmp(&crate::store::category_order(&right.question.category))
                })
                .then_with(|| left.question.chapter.cmp(&right.question.chapter))
                .then_with(|| left.question.index.cmp(&right.question.index))
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        for (record, group, user) in answer_values {
            let explanation = group
                .explanation
                .clone()
                .or_else(|| record.question.explanation.clone());
            wrong.push(WrongItem {
                record_id: record.id.clone(),
                paper: record.paper_key(),
                category: record.question.category.clone(),
                chapter: record.question.chapter.clone(),
                category_label: record.category_label().to_string(),
                chapter_label: record.chapter_label().to_string(),
                index: record.question.index,
                group: group.id.clone(),
                prompt: if group.prompt.trim().is_empty() {
                    record.question.prompt.clone()
                } else {
                    group.prompt.clone()
                },
                user: record.answer_value(group, user),
                answer: record.answer_value(group, group.answer.as_deref().unwrap_or("")),
                explanation,
            });
        }
    }

    PaperSummary {
        paper,
        title,
        level,
        exam,
        page_count,
        captured_pages,
        expected_groups,
        answered_groups,
        correct_groups,
        wrong_groups,
        accuracy: ratio(correct_groups, answered_groups),
        complete,
        last_captured_at,
        total_score,
        max_score: 180,
        pass_line,
        passed,
        sections,
        chapters,
        wrong,
    }
}

fn newer(record: &Record, current: Option<&Record>) -> bool {
    match current {
        Some(current) => compare_newer(record, current).is_gt(),
        None => true,
    }
}

fn compare_newer(a: &Record, b: &Record) -> std::cmp::Ordering {
    a.captured_at
        .cmp(&b.captured_at)
        .then_with(|| a.id.cmp(&b.id))
}

fn ratio(correct: usize, total: usize) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some(round4(correct as f64 / total as f64))
    }
}

fn round4(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

fn pass_line(level: &str) -> u32 {
    match level {
        "n1" => 100,
        "n2" => 90,
        "n3" => 95,
        "n4" => 90,
        "n5" => 80,
        _ => 90,
    }
}

fn section_meta(level: &str) -> Vec<SectionMeta> {
    match level {
        "n4" | "n5" => vec![
            SectionMeta {
                id: "language_reading",
                label: "语言知识与阅读",
                max: 120,
                min: 38,
            },
            SectionMeta {
                id: "listening",
                label: "听力",
                max: 60,
                min: 19,
            },
        ],
        _ => vec![
            SectionMeta {
                id: "language",
                label: "语言知识",
                max: 60,
                min: 19,
            },
            SectionMeta {
                id: "reading",
                label: "阅读",
                max: 60,
                min: 19,
            },
            SectionMeta {
                id: "listening",
                label: "听力",
                max: 60,
                min: 19,
            },
        ],
    }
}

fn section_for(level: &str, category: &str) -> &'static str {
    let base = match category {
        "reading" => "reading",
        "listening" => "listening",
        _ => "language",
    };
    if matches!(level, "n4" | "n5") && (base == "language" || base == "reading") {
        "language_reading"
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_combine_language_and_reading_for_n4() {
        assert_eq!(section_for("n2", "reading"), "reading");
        assert_eq!(section_for("n4", "reading"), "language_reading");
        assert_eq!(section_for("n5", "grammar"), "language_reading");
        assert_eq!(section_for("n4", "listening"), "listening");
    }

    #[test]
    fn pass_lines_match_official_thresholds() {
        assert_eq!(pass_line("n1"), 100);
        assert_eq!(pass_line("n2"), 90);
        assert_eq!(pass_line("n3"), 95);
        assert_eq!(pass_line("n4"), 90);
        assert_eq!(pass_line("n5"), 80);
    }

    #[test]
    fn n4_has_two_scoring_sections() {
        let meta = section_meta("n4");
        assert_eq!(meta.len(), 2);
        assert_eq!(meta[0].max, 120);
        assert_eq!(meta[0].min, 38);
        assert_eq!(meta[1].max, 60);
    }
}
