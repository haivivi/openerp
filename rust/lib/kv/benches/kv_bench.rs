use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use kv::{KVStore, OverlayKV, RedbStore};

fn bench_redb_set(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let store = RedbStore::open(&tmp.path().join("bench.redb")).unwrap();

    c.bench_function("redb_set", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let key = format!("bench:key:{}", i);
            store.set(black_box(&key), black_box(b"hello world")).unwrap();
            i += 1;
        });
    });
}

fn bench_redb_get(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let store = RedbStore::open(&tmp.path().join("bench.redb")).unwrap();

    // Pre-populate.
    for i in 0..1000 {
        let key = format!("bench:key:{:04}", i);
        store.set(&key, b"hello world").unwrap();
    }

    c.bench_function("redb_get", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let key = format!("bench:key:{:04}", i % 1000);
            let _ = store.get(black_box(&key)).unwrap();
            i += 1;
        });
    });
}

fn bench_redb_scan(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let store = RedbStore::open(&tmp.path().join("bench.redb")).unwrap();

    for i in 0..1000 {
        let key = format!("bench:key:{:04}", i);
        store.set(&key, b"hello world").unwrap();
    }

    c.bench_function("redb_scan_1000", |b| {
        b.iter(|| {
            let results = store.scan(black_box("bench:key:")).unwrap();
            assert_eq!(results.len(), 1000);
        });
    });
}

fn bench_overlay_get_file_layer(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let db = RedbStore::open(&tmp.path().join("bench.redb")).unwrap();
    let overlay = OverlayKV::new(db);

    // Populate file layer.
    for i in 0..1000 {
        let key = format!("config:model:{:04}", i);
        overlay.insert_file_entry(key, b"readonly value".to_vec());
    }

    c.bench_function("overlay_get_file_layer", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let key = format!("config:model:{:04}", i % 1000);
            let _ = overlay.get(black_box(&key)).unwrap();
            i += 1;
        });
    });
}

fn bench_overlay_get_db_layer(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let db = RedbStore::open(&tmp.path().join("bench.redb")).unwrap();
    let overlay = OverlayKV::new(db);

    // Populate DB layer.
    for i in 0..1000 {
        let key = format!("device:{:04}", i);
        overlay.set(&key, b"writable value").unwrap();
    }

    c.bench_function("overlay_get_db_layer", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let key = format!("device:{:04}", i % 1000);
            let _ = overlay.get(black_box(&key)).unwrap();
            i += 1;
        });
    });
}

criterion_group!(
    benches,
    bench_redb_set,
    bench_redb_get,
    bench_redb_scan,
    bench_overlay_get_file_layer,
    bench_overlay_get_db_layer,
);
criterion_main!(benches);
