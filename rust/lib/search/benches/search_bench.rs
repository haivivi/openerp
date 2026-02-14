use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use openerp_search::{SearchEngine, TantivyEngine};

fn bench_index_document(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let engine = TantivyEngine::open(tmp.path()).unwrap();

    c.bench_function("tantivy_index", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let id = format!("doc-{}", i);
            let mut fields = HashMap::new();
            fields.insert("name".to_string(), format!("Device H106 unit {}", i));
            fields.insert("sn".to_string(), format!("SN-{:08}", i));
            fields.insert("status".to_string(), "active".to_string());
            engine
                .index(black_box("devices"), black_box(&id), black_box(fields))
                .unwrap();
            i += 1;
        });
    });
}

fn bench_search(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let engine = TantivyEngine::open(tmp.path()).unwrap();

    // Pre-populate with 1000 documents.
    for i in 0..1000 {
        let id = format!("doc-{}", i);
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), format!("Device H106 unit {}", i));
        fields.insert("sn".to_string(), format!("SN-{:08}", i));
        fields.insert(
            "status".to_string(),
            if i % 3 == 0 {
                "shipped".to_string()
            } else {
                "active".to_string()
            },
        );
        engine.index("devices", &id, fields).unwrap();
    }

    c.bench_function("tantivy_search", |b| {
        b.iter(|| {
            let results = engine
                .search(black_box("devices"), black_box("H106"), black_box(10))
                .unwrap();
            assert!(!results.is_empty());
        });
    });
}

criterion_group!(benches, bench_index_document, bench_search);
criterion_main!(benches);
