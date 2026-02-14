use criterion::{black_box, criterion_group, criterion_main, Criterion};

use openerp_sql::{SQLStore, SqliteStore, Value};

fn bench_exec_insert(c: &mut Criterion) {
    let store = SqliteStore::open_in_memory().unwrap();
    store
        .exec(
            "CREATE TABLE bench (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, value REAL)",
            &[],
        )
        .unwrap();

    c.bench_function("sqlite_insert", |b| {
        b.iter(|| {
            store
                .exec(
                    "INSERT INTO bench (name, value) VALUES (?1, ?2)",
                    &[
                        Value::Text("item-bench".to_string()),
                        Value::Real(42.5),
                    ],
                )
                .unwrap();
        });
    });
}

fn bench_query_by_id(c: &mut Criterion) {
    let store = SqliteStore::open_in_memory().unwrap();
    store
        .exec(
            "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT, value REAL)",
            &[],
        )
        .unwrap();

    for i in 0..10000 {
        store
            .exec(
                "INSERT INTO bench (id, name, value) VALUES (?1, ?2, ?3)",
                &[
                    Value::Integer(i),
                    Value::Text(format!("item-{}", i)),
                    Value::Real(i as f64 * 1.5),
                ],
            )
            .unwrap();
    }

    let mut i = 0i64;
    c.bench_function("sqlite_query_by_id", |b| {
        b.iter(|| {
            let rows = store
                .query(
                    "SELECT id, name, value FROM bench WHERE id = ?1",
                    &[Value::Integer(black_box(i % 10000))],
                )
                .unwrap();
            assert_eq!(rows.len(), 1);
            i += 1;
        });
    });
}

fn bench_query_range(c: &mut Criterion) {
    let store = SqliteStore::open_in_memory().unwrap();
    store
        .exec(
            "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT, value REAL)",
            &[],
        )
        .unwrap();

    for i in 0..10000 {
        store
            .exec(
                "INSERT INTO bench (id, name, value) VALUES (?1, ?2, ?3)",
                &[
                    Value::Integer(i),
                    Value::Text(format!("item-{}", i)),
                    Value::Real(i as f64 * 1.5),
                ],
            )
            .unwrap();
    }

    let mut offset = 0i64;
    c.bench_function("sqlite_query_range_100", |b| {
        b.iter(|| {
            let rows = store
                .query(
                    "SELECT id, name, value FROM bench WHERE id >= ?1 LIMIT 100",
                    &[Value::Integer(black_box(offset % 9900))],
                )
                .unwrap();
            assert_eq!(rows.len(), 100);
            offset += 100;
        });
    });
}

criterion_group!(benches, bench_exec_insert, bench_query_by_id, bench_query_range);
criterion_main!(benches);
