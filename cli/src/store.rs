use crate::model::{ExportFile, Record};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    #[serde(default = "store_version")]
    pub version: u32,
    #[serde(default)]
    pub records: Vec<Record>,
}

fn store_version() -> u32 {
    1
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: 1,
            records: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ImportStats {
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub files: usize,
}

pub fn resolve_store_path(cli_store: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = cli_store {
        return Ok(path);
    }
    if let Some(path) = env::var_os("JALAQUERY_STORE") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("jalaquery").join("store.json"));
    }
    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("jalaquery")
            .join("store.json"));
    }
    bail!("无法确定数据文件位置，请设置 HOME、XDG_DATA_HOME 或使用 --store")
}

pub fn load(path: &Path) -> Result<Store> {
    if !path.exists() {
        return Ok(Store::default());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("读取数据文件失败：{}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Store::default());
    }
    let store: Store = serde_json::from_str(&text)
        .with_context(|| format!("数据文件格式不正确：{}", path.display()))?;
    if store.version != 1 {
        bail!("不支持的数据文件版本：{}", store.version);
    }
    Ok(store)
}

pub fn save(path: &Path, store: &Store) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建数据目录失败：{}", parent.display()))?;
    }
    let data = serde_json::to_vec(store).context("序列化数据失败")?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, data).with_context(|| format!("写入数据文件失败：{}", temp.display()))?;
    fs::rename(&temp, path).with_context(|| format!("替换数据文件失败：{}", path.display()))?;
    Ok(())
}

pub fn import_paths(paths: &[PathBuf], store_path: &Path) -> Result<ImportStats> {
    if paths.is_empty() {
        bail!("请提供至少一个导出文件");
    }
    let files = expand_paths(paths)?;
    if files.is_empty() {
        bail!("没有找到 JSON 文件");
    }
    let mut store = load(store_path)?;
    let mut records: BTreeMap<String, Record> = store
        .records
        .drain(..)
        .map(|record| (record.id.clone(), record))
        .collect();
    let mut stats = ImportStats {
        files: files.len(),
        ..ImportStats::default()
    };
    for file in &files {
        let (exported, skipped) = read_export(file)?;
        stats.skipped += skipped;
        for record in exported {
            if records.insert(record.id.clone(), record).is_some() {
                stats.updated += 1;
            } else {
                stats.added += 1;
            }
        }
    }
    let mut merged: Vec<Record> = records.into_values().collect();
    sort_records(&mut merged);
    store.records = merged;
    save(store_path, &store)?;
    Ok(stats)
}

fn expand_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut found = Vec::new();
            for entry in
                fs::read_dir(path).with_context(|| format!("读取目录失败：{}", path.display()))?
            {
                let entry = entry?;
                let item = entry.path();
                if item.extension().and_then(|ext| ext.to_str()) == Some("json") {
                    found.push(item);
                }
            }
            found.sort();
            files.extend(found);
        } else if path.is_file() {
            files.push(path.clone());
        } else {
            bail!("文件不存在：{}", path.display());
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn read_export(path: &Path) -> Result<(Vec<Record>, usize)> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("读取导出文件失败：{}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("导出文件不是合法 JSON：{}", path.display()))?;
    let records: Vec<Record> = if value.is_array() {
        serde_json::from_value(value)
            .with_context(|| format!("导出文件格式不正确：{}", path.display()))?
    } else {
        if let Some(format) = value.get("format").and_then(Value::as_str) {
            if format != "jalaplus.export" {
                bail!("不支持的导出格式：{}（{}）", format, path.display());
            }
        }
        if let Some(version) = value.get("version").and_then(Value::as_u64) {
            if version != 1 {
                bail!("不支持的导出版本：{}（{}）", version, path.display());
            }
        }
        let export: ExportFile = serde_json::from_value(value)
            .with_context(|| format!("导出文件格式不正确：{}", path.display()))?;
        export.records
    };
    let total = records.len();
    let mut valid = Vec::with_capacity(total);
    for mut record in records {
        record.normalize();
        if record.is_valid() {
            valid.push(record);
        }
    }
    let skipped = total - valid.len();
    Ok((valid, skipped))
}

pub fn sort_records(records: &mut [Record]) {
    records.sort_by(|a, b| {
        a.paper_key()
            .cmp(&b.paper_key())
            .then_with(|| {
                category_order(&a.question.category).cmp(&category_order(&b.question.category))
            })
            .then_with(|| {
                chapter_order(&a.question.category, &a.question.chapter)
                    .cmp(&chapter_order(&b.question.category, &b.question.chapter))
            })
            .then_with(|| a.question.index.cmp(&b.question.index))
            .then_with(|| a.captured_at.cmp(&b.captured_at))
            .then_with(|| a.id.cmp(&b.id))
    });
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

pub fn store_size(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
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
