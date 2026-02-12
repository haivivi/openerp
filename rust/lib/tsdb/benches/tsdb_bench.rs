use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use openerp_tsdb::{LogEntry, LogQuery, TsDb, WalEngine};

fn make_entry(ts: u64, log_type: &str, fw: &str) -> LogEntry {
    let mut labels = HashMap::new();
    labels.insert("type".to_string(), log_type.to_string());
    labels.insert("fw".to_string(), fw.to_string());

    let data = serde_json::json!({
        "battery": 85,
        "rssi": -42,
        "uptime": ts / 1_000_000_000,
    });

    LogEntry {
        ts,
        labels,
        data: serde_json::to_vec(&data).unwrap(),
    }
}

fn bench_write_single(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let engine = WalEngine::open(tmp.path()).unwrap();

    c.bench_function("tsdb_write_single", |b| {
        let mut ts = 1_000_000_000u64;
        b.iter(|| {
            let entry = make_entry(ts, "heartbeat", "1.2.3");
            engine
                .write(black_box("device-001"), black_box(entry))
                .unwrap();
            ts += 1_000_000; // 1ms increment
        });
    });
}

fn bench_write_batch_100(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let engine = WalEngine::open(tmp.path()).unwrap();

    c.bench_function("tsdb_write_batch_100", |b| {
        let mut base_ts = 1_000_000_000u64;
        b.iter(|| {
            let entries: Vec<LogEntry> = (0..100)
                .map(|i| make_entry(base_ts + i * 1_000_000, "heartbeat", "1.2.3"))
                .collect();
            engine
                .write_batch(black_box("device-002"), black_box(entries))
                .unwrap();
            base_ts += 100_000_000;
        });
    });
}

fn bench_query_hot(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let engine = WalEngine::open(tmp.path()).unwrap();

    // Write 1000 entries (stays in WAL, below compaction threshold).
    for i in 0..1000 {
        let entry = make_entry(
            1_000_000_000 + i * 1_000_000,
            if i % 3 == 0 { "error" } else { "heartbeat" },
            "1.2.3",
        );
        engine.write("device-003", entry).unwrap();
    }

    c.bench_function("tsdb_query_hot_50", |b| {
        b.iter(|| {
            let query = LogQuery {
                stream: "device-003".to_string(),
                labels: {
                    let mut m = HashMap::new();
                    m.insert("type".to_string(), "heartbeat".to_string());
                    m
                },
                limit: 50,
                desc: true,
                start: None,
                end: None,
            };
            let results = engine.query(black_box(&query)).unwrap();
            assert!(!results.is_empty());
        });
    });
}

fn bench_query_with_time_range(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let engine = WalEngine::open(tmp.path()).unwrap();

    for i in 0..1000 {
        let entry = make_entry(1_000_000_000 + i * 1_000_000, "heartbeat", "1.2.3");
        engine.write("device-004", entry).unwrap();
    }

    c.bench_function("tsdb_query_time_range", |b| {
        b.iter(|| {
            let query = LogQuery {
                stream: "device-004".to_string(),
                labels: HashMap::new(),
                limit: 50,
                desc: false,
                start: Some(1_000_000_000 + 200 * 1_000_000),
                end: Some(1_000_000_000 + 400 * 1_000_000),
            };
            let results = engine.query(black_box(&query)).unwrap();
            assert!(!results.is_empty());
        });
    });
}

criterion_group!(
    benches,
    bench_write_single,
    bench_write_batch_100,
    bench_query_hot,
    bench_query_with_time_range,
);
criterion_main!(benches);
