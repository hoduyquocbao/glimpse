use criterion::{criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::{Cursor, Write};
use memmap2::Mmap;

// Hàm tạo file nhị phân lớn giả lập (nếu chưa có)
fn create_large_file(path: &str, packet_count: usize) {
    let mut file = File::create(path).unwrap();
    for i in 0..packet_count {
        // Header: version (u16), length (u16)
        let version: u16 = (i % 256) as u16;
        let length: u16 = 3;
        file.write_all(&version.to_be_bytes()).unwrap();
        file.write_all(&length.to_be_bytes()).unwrap();
        file.write_all(b"abc").unwrap();
    }
    file.flush().unwrap();
}

fn bench_mmap_sequential(c: &mut Criterion) {
    let path = "bench_seq.bin";
    create_large_file(path, 100_000);
    let file = File::open(path).unwrap();
    let mmap = unsafe { Mmap::map(&file).unwrap() };
    c.bench_function("mmap_sequential", |b| {
        b.iter(|| {
            let mut count = 0;
            let mut buf = &mmap[..];
            while !buf.is_empty() {
                match storage::scan(buf) {
                    Ok((_, rest)) => {
                        count += 1;
                        buf = rest;
                    }
                    Err(_) => break,
                }
            }
            std::hint::black_box(count)
        })
    });
}

fn bench_mmap_parallel(c: &mut Criterion) {
    let path = "bench_par.bin";
    create_large_file(path, 100_000);
    let file = File::open(path).unwrap();
    let mmap = unsafe { Mmap::map(&file).unwrap() };
    c.bench_function("mmap_parallel", |b| {
        b.iter(|| {
            let count = storage::batch(&mmap);
            std::hint::black_box(count)
        })
    });
}

fn bench_streaming_processor(c: &mut Criterion) {
    let path = "bench_stream.bin";
    create_large_file(path, 100_000);
    let file = File::open(path).unwrap();
    let mmap = unsafe { Mmap::map(&file).unwrap() };
    let cursor = Cursor::new(&mmap[..]);
    c.bench_function("streaming_processor", |b| {
        b.iter(|| {
            let mut proc = storage::stream(cursor.clone());
            let mut count = 0;
            while let Some(_packet) = proc.next() {
                count += 1;
            }
            std::hint::black_box(count)
        })
    });
}

criterion_group!(benches, bench_mmap_sequential, bench_mmap_parallel, bench_streaming_processor);
criterion_main!(benches); 