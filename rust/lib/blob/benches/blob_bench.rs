use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use blob::{BlobStore, FileStore};

fn bench_put_1kb(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let store = FileStore::open(tmp.path()).unwrap();
    let data = vec![0xABu8; 1024];

    c.bench_function("blob_put_1kb", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let key = format!("bench/file-{}.bin", i);
            store.put(black_box(&key), black_box(&data)).unwrap();
            i += 1;
        });
    });
}

fn bench_put_1mb(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let store = FileStore::open(tmp.path()).unwrap();
    let data = vec![0xABu8; 1024 * 1024];

    c.bench_function("blob_put_1mb", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let key = format!("bench/file-{}.bin", i);
            store.put(black_box(&key), black_box(&data)).unwrap();
            i += 1;
        });
    });
}

fn bench_get_1kb(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let store = FileStore::open(tmp.path()).unwrap();
    let data = vec![0xABu8; 1024];

    for i in 0..1000 {
        let key = format!("bench/file-{}.bin", i);
        store.put(&key, &data).unwrap();
    }

    c.bench_function("blob_get_1kb", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let key = format!("bench/file-{}.bin", i % 1000);
            let _ = store.get(black_box(&key)).unwrap();
            i += 1;
        });
    });
}

fn bench_list(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let store = FileStore::open(tmp.path()).unwrap();

    for i in 0..500 {
        let key = format!("firmware/h106/v{}.bin", i);
        store.put(&key, b"fw").unwrap();
    }
    for i in 0..500 {
        let key = format!("firmware/h2xx/v{}.bin", i);
        store.put(&key, b"fw").unwrap();
    }

    c.bench_function("blob_list_500", |b| {
        b.iter(|| {
            let results = store.list(black_box("firmware/h106/")).unwrap();
            assert_eq!(results.len(), 500);
        });
    });
}

criterion_group!(benches, bench_put_1kb, bench_put_1mb, bench_get_1kb, bench_list);
criterion_main!(benches);
