use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("jalaquery-test-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn run(input: &Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_jalaquery"))
        .arg("--input")
        .arg(input)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "command failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn group(id: &str, answer: &str) -> Value {
    json!({
        "id": id,
        "prompt": format!("小题 {id}"),
        "options": [
            {"id": "a", "label": "选项A"},
            {"id": "b", "label": "选项B"}
        ],
        "answer": answer,
        "explanation": format!("解析 {id}"),
    })
}

fn record(
    id: &str,
    index: u32,
    chapter: &str,
    groups: Vec<Value>,
    answers: Value,
    timing: Option<Value>,
) -> Value {
    let category = if chapter == "word-formation" {
        "vocabulary"
    } else {
        "grammar"
    };
    let kind = if timing.is_some() {
        "answer"
    } else {
        "question"
    };
    json!({
        "id": id,
        "kind": kind,
        "capturedAt": format!("2026-10-20T10:0{index}:00.000Z"),
        "batchId": null,
        "sessionId": timing.as_ref().map(|_| "session-test"),
        "paperRef": "n2/2020-12",
        "questionRef": format!("n2/2020-12/{category}/{chapter}/{index}"),
        "paper": {
            "level": "n2",
            "exam": "2020-12",
            "title": "JLPT N2・2020年12月",
            "locale": "zh"
        },
        "question": {
            "category": category,
            "chapter": chapter,
            "index": index,
            "labels": {
                "category": if chapter == "word-formation" { "词汇" } else { "语法" },
                "chapter": if chapter == "word-formation" { "构词" } else { "语法形式判断" }
            },
            "prompt": format!("第{index}题题干"),
            "groups": groups,
            "explanation": "整体解析",
            "transcript": null,
            "passage": null,
            "audio": null,
            "optionsHidden": false,
            "siteStats": {"correct": 1, "wrong": 1}
        },
        "attempt": {
            "answers": answers,
            "answeredGroups": 2,
            "totalGroups": 2,
            "checked": true
        },
        "timing": timing,
        "access": {"explanation": "full", "transcript": "absent"}
    })
}

#[test]
fn reads_exported_records_without_a_store() {
    let dir = temp_dir();
    let export_path = dir.join("export.json");
    fs::write(
        dir.join("storage.json"),
        serde_json::to_vec(&json!({
            "format": "jalaplus.storage",
            "version": 1,
            "localStorage": {},
            "sessionStorage": {}
        }))
        .unwrap(),
    )
    .unwrap();
    let export = json!({
        "format": "jalaplus.export",
        "version": 2,
        "records": [
            record(
                "record-1",
                1,
                "word-formation",
                vec![group("g1", "a"), group("g2", "a")],
                json!({"g1": "a", "g2": "b"}), // 1 对 1 错
                None
            ),
            record(
                "record-2",
                2,
                "grammar-form",
                vec![group("g1", "a")],
                json!({"g1": "a"}), // 1 对
                Some(json!({
                    "sessionId": "session-test",
                    "pageLoadedAt": "2026-10-20T10:01:50.000Z",
                    "answeredAt": "2026-10-20T10:02:00.000Z",
                    "pageMs": 10000,
                    "sessionStartedAt": "2026-10-20T10:01:00.000Z",
                    "sessionMs": 60000,
                    "navigation": "spa"
                }))
            ),
        ],
        "batches": [{
            "id": "batch-test",
            "paperRef": "n2/2020-12",
            "startedAt": "2026-10-20T10:00:00.000Z",
            "endedAt": "2026-10-20T10:10:00.000Z",
            "complete": true,
            "pageCount": 2,
            "groupCount": 3,
            "chapterPages": {"vocabulary/word-formation": 1, "grammar/grammar-form": 1},
            "chapterGroups": {"vocabulary/word-formation": 2, "grammar/grammar-form": 1}
        }],
        "sessions": [{
            "id": "session-test",
            "startedAt": "2026-10-20T10:01:00.000Z",
            "endedAt": null
        }]
    });
    fs::write(&export_path, serde_json::to_vec(&export).unwrap()).unwrap();

    let overview = run(&export_path, &["overview"]);
    assert_eq!(overview["papers"][0]["paper"], "n2/2020-12");
    assert_eq!(overview["papers"][0]["answeredGroups"], 3);
    assert_eq!(overview["papers"][0]["correctGroups"], 2);
    assert_eq!(overview["papers"][0]["wrongGroups"], 1);

    let paper = run(&export_path, &["paper", "n2/2020-12"]);
    assert_eq!(paper["expectedGroups"], 3);
    assert_eq!(paper["answeredGroups"], 3);
    assert_eq!(paper["correctGroups"], 2);
    assert_eq!(paper["totalScore"], 40);
    assert_eq!(paper["passLine"], 90);
    assert_eq!(paper["passed"], false);
    assert_eq!(paper["sections"][0]["label"], "语言知识");
    assert_eq!(paper["sections"][0]["score"], 40);
    assert_eq!(paper["chapters"].as_array().unwrap().len(), 2);

    let with_site = run(&export_path, &["paper", "n2/2020-12", "--site-stats"]);
    assert_eq!(with_site["siteStats"]["correct"], 2);
    assert_eq!(with_site["siteStats"]["wrong"], 2);
    assert_eq!(with_site["siteStats"]["questions"], 2);

    let wrong = run(&export_path, &["wrong", "n2/2020-12"]);
    assert_eq!(wrong["count"], 1);
    assert_eq!(wrong["items"][0]["user"]["id"], "b");
    assert_eq!(wrong["items"][0]["answer"]["id"], "a");
    assert_eq!(wrong["items"][0]["categoryLabel"], "词汇");
    assert_eq!(wrong["items"][0]["explanation"], "解析 g2");

    let site_wrong = run(&export_path, &["wrong", "n2/2020-12", "--site-stats"]);
    assert_eq!(site_wrong["count"], 2);

    let shown = run(
        &export_path,
        &["show", "n2/2020-12/vocabulary/word-formation/1"],
    );
    assert_eq!(shown["records"].as_array().unwrap().len(), 1);
    assert_eq!(shown["records"][0]["attempt"]["answers"]["g1"], "a");

    let stats = run(&export_path, &["stats", "n2/2020-12", "--by", "chapter"]);
    assert_eq!(stats["rows"].as_array().unwrap().len(), 2);

    let site_stats = run(
        &export_path,
        &["stats", "n2/2020-12", "--by", "chapter", "--site-stats"],
    );
    assert_eq!(site_stats["siteStats"], true);
    assert_eq!(site_stats["rows"].as_array().unwrap().len(), 2);

    let timing = run(&export_path, &["timing"]);
    assert_eq!(timing["count"], 1);
    assert_eq!(timing["sessions"], 1);
    assert_eq!(timing["rows"][0]["pageMs"], 10000);

    let directory = run(&dir, &["paper", "n2/2020-12"]);
    assert_eq!(directory["answeredGroups"], 3);

    fs::remove_dir_all(dir).unwrap();
}
