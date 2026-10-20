use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub exported_at: Option<String>,
    #[serde(default)]
    pub records: Vec<Record>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub captured_at: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub batch_id: Option<String>,
    #[serde(default)]
    pub paper: Paper,
    #[serde(default)]
    pub question: Question,
    #[serde(default)]
    pub attempt: Attempt,
    #[serde(default)]
    pub access: Access,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Paper {
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub exam: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub page_count: Option<usize>,
    #[serde(default)]
    pub chapter_pages: Option<BTreeMap<String, usize>>,
    #[serde(default)]
    pub group_count: Option<usize>,
    #[serde(default)]
    pub chapter_groups: Option<BTreeMap<String, usize>>,
    #[serde(default)]
    pub complete: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub chapter: String,
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub labels: Labels,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub passage: Option<Passage>,
    #[serde(default)]
    pub audio: Option<String>,
    #[serde(default)]
    pub options_hidden: bool,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub transcript: Option<Transcript>,
    #[serde(default)]
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Labels {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub chapter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Passage {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub translation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub options: Vec<OptionItem>,
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default)]
    pub audio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OptionItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub sentences: Vec<Sentence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    #[serde(default)]
    pub role_id: i32,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Sentence {
    #[serde(default)]
    pub start_ms: u64,
    #[serde(default)]
    pub end_ms: u64,
    #[serde(default)]
    pub role_id: i32,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    #[serde(default)]
    pub answers: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub answered_groups: u32,
    #[serde(default)]
    pub total_groups: u32,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub site_stats: Option<SiteStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SiteStats {
    #[serde(default)]
    pub correct: u32,
    #[serde(default)]
    pub wrong: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Access {
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub transcript: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerValue {
    pub id: String,
    pub label: String,
    pub number: usize,
}

impl Record {
    pub fn paper_key(&self) -> String {
        format!("{}/{}", self.paper.level, self.paper.exam)
    }

    pub fn question_key(&self) -> String {
        format!(
            "{}/{}/{}/{}/{}",
            self.paper.level,
            self.paper.exam,
            self.question.category,
            self.question.chapter,
            self.question.index
        )
    }

    pub fn group_count(&self) -> usize {
        self.question.groups.len()
    }

    pub fn has_answers(&self) -> bool {
        self.attempt
            .answers
            .as_ref()
            .map(|answers| !answers.is_empty())
            .unwrap_or(false)
    }

    pub fn is_valid(&self) -> bool {
        !self.id.trim().is_empty()
            && !self.paper.level.trim().is_empty()
            && !self.paper.exam.trim().is_empty()
            && !self.question.category.trim().is_empty()
            && !self.question.chapter.trim().is_empty()
            && !self.question.groups.is_empty()
    }

    pub fn normalize(&mut self) {
        self.id = self.id.trim().to_string();
        self.captured_at = self.captured_at.trim().to_string();
        self.paper.level = self.paper.level.trim().to_lowercase();
        self.paper.exam = self.paper.exam.trim().to_string();
        self.paper.title = self
            .paper
            .title
            .take()
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty());
        self.question.category = self.question.category.trim().to_string();
        self.question.chapter = self.question.chapter.trim().to_string();
        self.question.labels.category = self.question.labels.category.trim().to_string();
        self.question.labels.chapter = self.question.labels.chapter.trim().to_string();
        for group in &mut self.question.groups {
            group.id = group.id.trim().to_string();
            group.answer = group
                .answer
                .take()
                .map(|answer| answer.trim().to_string())
                .filter(|answer| !answer.is_empty());
            for option in &mut group.options {
                option.id = option.id.trim().to_string();
            }
        }
        if let Some(answers) = &mut self.attempt.answers {
            *answers = answers
                .iter()
                .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
                .collect();
        }
    }

    pub fn display_title(&self) -> String {
        self.paper.title.clone().unwrap_or_else(|| {
            format!(
                "JLPT {}・{}",
                self.paper.level.to_uppercase(),
                self.paper.exam
            )
        })
    }

    pub fn category_label(&self) -> &str {
        if !self.question.labels.category.is_empty() {
            &self.question.labels.category
        } else {
            &self.question.category
        }
    }

    pub fn chapter_label(&self) -> &str {
        if !self.question.labels.chapter.is_empty() {
            &self.question.labels.chapter
        } else {
            &self.question.chapter
        }
    }

    pub fn answer_value(&self, group: &Group, id: &str) -> AnswerValue {
        match group.options.iter().position(|option| option.id == id) {
            Some(index) => AnswerValue {
                id: id.to_string(),
                label: group.options[index].label.clone(),
                number: index + 1,
            },
            None => AnswerValue {
                id: id.to_string(),
                label: id.to_string(),
                number: 0,
            },
        }
    }
}
