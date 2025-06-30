use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::io::Cursor;
use storage::{batch, scan, stream};

/// Tạo một buffer trong bộ nhớ chứa `count` packets giả lập.
/// Mỗi packet có 4-byte header (version=1, length=60) và 60-byte payload.
fn dataset(count: usize) -> Vec<u8> {
    const PAYLOAD_SIZE: u16 = 60;
    const PACKET_SIZE: usize = (4 + PAYLOAD_SIZE) as usize;
    let mut buffer = Vec::with_capacity(count * PACKET_SIZE);
    for _ in 0..count {
        // Header: version (u16), length (u16)
        let version: u16 = 1;
        buffer.extend_from_slice(&version.to_be_bytes());
        buffer.extend_from_slice(&PAYLOAD_SIZE.to_be_bytes());
        // Payload
        buffer.extend_from_slice(&[0u8; PAYLOAD_SIZE as usize]);
    }
    buffer
}

fn bench(c: &mut Criterion) {
    // Chuẩn bị dữ liệu chung cho tất cả các benchmark
    let data = dataset(100_000);

    // Tạo một nhóm benchmark để so sánh các hàm
    let mut group = c.benchmark_group("storage");

    // Benchmark hàm `scan` (đo hiệu năng đọc một packet đơn lẻ)
    group.bench_function("scan", |b| {
        b.iter(|| {
            let _ = scan(black_box(&data));
        })
    });

    // Benchmark hàm `batch` (xử lý song song toàn bộ buffer)
    group.bench_function("batch", |b| {
        b.iter(|| {
            let count = batch(black_box(&data));
            black_box(count);
        })
    });

    // Benchmark hàm `stream` (xử lý qua một IO Reader)
    group.bench_function("stream", |b| {
        b.iter(|| {
            let cursor = Cursor::new(black_box(&data));
            let proc = stream(cursor);
            let count = proc.count();
            black_box(count);
        })
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches); 