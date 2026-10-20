use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
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

fn run(store: &PathBuf, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_jalaquery"))
        .arg("--store")
        .arg(store)
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

fn record(id: &str, index: u32, chapter: &str, groups: Vec<Value>, answers: Value) -> Value {
    json!({
        "id": id,
        "capturedAt": format!("2026-10-20T10:0{index}:00.000Z"),
        "source": "exam",
        "batchId": "batch-test",
        "paper": {
            "level": "n2",
            "exam": "2020-12",
            "title": "JLPT N2・2020年12月",
            "locale": "zh",
            "pageCount": 2,
            "chapterPages": {"vocabulary/word-formation": 1, "grammar/grammar-form": 1},
            "groupCount": 3,
            "chapterGroups": {"vocabulary/word-formation": 2, "grammar/grammar-form": 1},
            "complete": true
        },
        "question": {
            "category": if chapter == "word-formation" { "vocabulary" } else { "grammar" },
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
            "optionsHidden": false
        },
        "attempt": {
            "answers": answers,
            "answeredGroups": 2,
            "totalGroups": 2,
            "checked": true,
            "siteStats": {"correct": 1, "wrong": 1}
        },
        "access": {"explanation": "full", "transcript": "absent"}
    })
}

#[test]
fn imports_and_reports_exported_records() {
    let dir = temp_dir();
    let export_path = dir.join("export.json");
    let store_path = dir.join("store.json");
    let export = json!({
        "format": "jalaplus.export",
        "version": 1,
        "records": [
            record(
                "record-1",
                1,
                "word-formation",
                vec![group("g1", "a"), group("g2", "a")],
                json!({"g1": "a", "g2": "b"}), // 1 对 1 错
            ),
            record(
                "record-2",
                2,
                "grammar-form",
                vec![group("g1", "a")],
                json!({"g1": "a"}), // 1 对
            ),
        ],
    });
    fs::write(&export_path, serde_json::to_vec(&export).unwrap()).unwrap();

    let imported = run(&store_path, &["import", export_path.to_str().unwrap()]);
    assert_eq!(imported["added"], 2);
    assert_eq!(imported["skipped"], 0);

    let overview = run(&store_path, &["overview"]);
    assert_eq!(overview["papers"][0]["paper"], "n2/2020-12");
    assert_eq!(overview["papers"][0]["answeredGroups"], 3);
    assert_eq!(overview["papers"][0]["correctGroups"], 2);
    assert_eq!(overview["papers"][0]["wrongGroups"], 1);

    let paper = run(&store_path, &["paper", "n2/2020-12"]);
    assert_eq!(paper["expectedGroups"], 3);
    assert_eq!(paper["answeredGroups"], 3);
    assert_eq!(paper["correctGroups"], 2);
    assert_eq!(paper["totalScore"], 40);
    assert_eq!(paper["passLine"], 90);
    assert_eq!(paper["passed"], false);
    assert_eq!(paper["sections"][0]["label"], "语言知识");
    assert_eq!(paper["sections"][0]["score"], 40);
    assert_eq!(paper["chapters"].as_array().unwrap().len(), 2);

    let wrong = run(&store_path, &["wrong", "n2/2020-12"]);
    assert_eq!(wrong["count"], 1);
    assert_eq!(wrong["items"][0]["user"]["id"], "b");
    assert_eq!(wrong["items"][0]["answer"]["id"], "a");
    assert_eq!(wrong["items"][0]["categoryLabel"], "词汇");
    assert_eq!(wrong["items"][0]["explanation"], "解析 g2");

    let shown = run(
        &store_path,
        &["show", "n2/2020-12/vocabulary/word-formation/1"],
    );
    assert_eq!(shown["records"].as_array().unwrap().len(), 1);
    assert_eq!(shown["records"][0]["attempt"]["answers"]["g1"], "a");

    let stats = run(&store_path, &["stats", "n2/2020-12", "--by", "chapter"]);
    assert_eq!(stats["rows"].as_array().unwrap().len(), 2);

    let imported_again = run(&store_path, &["import", export_path.to_str().unwrap()]);
    assert_eq!(imported_again["added"], 0);
    assert_eq!(imported_again["updated"], 2);

    fs::remove_dir_all(dir).unwrap();
}
