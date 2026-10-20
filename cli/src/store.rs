use crate::model::{Batch, ExportFile, Record, Session};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Dataset {
    pub records: Vec<Record>,
    pub batches: Vec<Batch>,
    pub sessions: Vec<Session>,
}

impl Dataset {
    pub fn batch_for(&self, paper: &str) -> Option<&Batch> {
        self.batches
            .iter()
            .filter(|batch| batch.paper_ref == paper)
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
    }
}

pub fn load_inputs(paths: &[PathBuf]) -> Result<Dataset> {
    if paths.is_empty() {
        bail!("请使用 --input 提供至少一个导出文件或目录");
    }

    let files = collect_files(paths)?;
    if files.is_empty() {
        bail!("没有找到可读取的 JSON 文件");
    }

    let mut records: BTreeMap<String, Record> = BTreeMap::new();
    let mut batches: BTreeMap<String, Batch> = BTreeMap::new();
    let mut sessions: BTreeMap<String, Session> = BTreeMap::new();

    for path in files {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("读取导出文件失败：{}", path.display()))?;
        let export: ExportFile = serde_json::from_str(&text)
            .with_context(|| format!("解析导出文件失败：{}", path.display()))?;
        if export.version != Some(2) {
            bail!("导出文件版本不受支持：{}", path.display());
        }
        for mut record in export.records {
            record.normalize();
            if !record.is_valid() {
                bail!("记录格式不完整：{}", path.display());
            }
            records.insert(record.id.clone(), record);
        }
        for batch in export.batches {
            if !batch.id.trim().is_empty() {
                batches.insert(batch.id.clone(), batch);
            }
        }
        for session in export.sessions {
            if !session.id.trim().is_empty() {
                sessions.insert(session.id.clone(), session);
            }
        }
    }

    Ok(Dataset {
        records: records.into_values().collect(),
        batches: batches.into_values().collect(),
        sessions: sessions.into_values().collect(),
    })
}

fn collect_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    for path in paths {
        collect_path(path, &mut files)?;
    }
    Ok(files.into_iter().collect())
}

fn collect_path(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("无法读取路径：{}", path.display()))?;
    if metadata.is_file() {
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.insert(path.to_path_buf());
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        bail!("不支持的路径：{}", path.display());
    }
    for entry in fs::read_dir(path).with_context(|| format!("读取目录失败：{}", path.display()))?
    {
        let entry = entry?;
        collect_path(&entry.path(), files)?;
    }
    Ok(())
}

pub fn chapter_order(category: &str, chapter: &str) -> u8 {
    const ORDER: [(&str, &str); 19] = [
        ("characters", "kanji-reading"),
        ("characters", "orthography"),
        ("vocabulary", "word-formation"),
        ("vocabulary", "contextually-defined"),
        ("vocabulary", "paraphrase"),
        ("vocabulary", "usage"),
        ("grammar", "grammar-form"),
        ("grammar", "sentence-composition"),
        ("grammar", "text-grammar"),
        ("reading", "short-passage"),
        ("reading", "medium-passage"),
        ("reading", "integrated-comparison"),
        ("reading", "thematic-passage"),
        ("reading", "information-retrieval"),
        ("listening", "task-based-comprehension"),
        ("listening", "key-points"),
        ("listening", "general-outline"),
        ("listening", "quick-response"),
        ("listening", "integrated-comprehension"),
    ];
    ORDER
        .iter()
        .position(|(left, right)| *left == category && *right == chapter)
        .map(|index| index as u8)
        .unwrap_or(99)
}

pub fn category_order(category: &str) -> u8 {
    match category {
        "characters" => 0,
        "vocabulary" => 1,
        "grammar" => 2,
        "reading" => 3,
        "listening" => 4,
        _ => 9,
    }
}

pub fn paper_keys(records: &[Record]) -> Vec<String> {
    let keys: BTreeSet<String> = records.iter().map(Record::paper_key).collect();
    keys.into_iter().collect()
}

pub fn normalize_paper(input: &str) -> Result<String> {
    let normalized = input.trim().to_lowercase().replace(' ', "/");
    let mut parts = normalized.split('/');
    let level = parts.next().unwrap_or("");
    let exam = parts.next().unwrap_or("");
    if parts.next().is_some()
        || !matches!(level, "n1" | "n2" | "n3" | "n4" | "n5")
        || exam.is_empty()
    {
        bail!("试卷格式应为 n2/2020-12，例如 n2/2020-12");
    }
    Ok(format!("{level}/{exam}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_input_is_normalized() {
        assert_eq!(normalize_paper("N2/2020-12").unwrap(), "n2/2020-12");
        assert_eq!(normalize_paper("n2 2020-12").unwrap(), "n2/2020-12");
        assert!(normalize_paper("2020-12").is_err());
        assert!(normalize_paper("n2/2020-12/extra").is_err());
    }

    #[test]
    fn chapter_order_follows_site_order() {
        assert!(
            chapter_order("characters", "kanji-reading")
                < chapter_order("vocabulary", "word-formation")
        );
        assert!(
            chapter_order("reading", "short-passage") < chapter_order("reading", "medium-passage")
        );
        assert_eq!(chapter_order("unknown", "unknown"), 99);
    }
}
