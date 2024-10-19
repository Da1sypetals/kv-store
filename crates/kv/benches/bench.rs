use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kv::store::utils::TempStore;

fn bench_put(c: &mut Criterion) {
    let (_raii, store) = TempStore::init(201);

    c.bench_function("bench-put", |b| {
        let i = 124;
        b.iter(|| {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            store.put(key.into(), val.into()).unwrap();
        });
    });
}

fn bench_batched_put(c: &mut Criterion) {
    let (_raii, store) = TempStore::init(201);

    c.bench_function("bench-batched-put", |b| {
        b.iter(|| {
            let batch = store.new_batched();
            for i in 0..64 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                batch.put(key.into(), val.into()).unwrap();
            }
            batch.commit().unwrap();
        });
    });
}

fn bench_mixed(c: &mut Criterion) {
    let (_raii, store) = TempStore::init(201);

    c.bench_function("bench-batched-put", |b| {
        b.iter(|| {
            let batch = store.new_batched();
            for i in 0..64 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                batch.put(key.into(), val.into()).unwrap();
            }
            batch.commit().unwrap();

            for i in 16..48 {
                let key = format!("{}", i);
                let _val = store.get(key.into()).unwrap();
            }

            let batch = store.new_batched();
            for i in 16..48 {
                let key = format!("{}", i);
                batch.delete(key.into()).unwrap();
            }
            batch.commit().unwrap();
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_put, bench_batched_put, bench_mixed
}
criterion_main!(benches);
