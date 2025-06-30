use std::io::Read;

use glimpse::*;
use rayon::prelude::*;

/// Đọc một packet từ buffer, trả về packet và phần còn lại.
pub fn scan<'a>(buf: &'a [u8]) -> Result<(Packet<'a>, &'a [u8]), Fault> {
    Parser::<Packet>::read(buf)
}

/// Phân tích song song các chunk của mmap, trả về tổng số packet.
pub fn batch<'a>(mmap: &'a [u8]) -> usize {
    let chunk = 4096;
    let list: Vec<&'a [u8]> = mmap.chunks(chunk).collect();
    list.par_iter().map(|part| {
        let mut buf = *part;
        let mut sum = 0;
        while !buf.is_empty() {
            match Parser::<Packet>::read(buf) {
                Ok((_, rest)) => {
                    sum += 1;
                    buf = rest;
                }
                Err(_) => break,
            }
        }
        sum
    }).sum()
}

/// Tạo processor streaming cho reader.
pub fn stream<'a, R: Read + Send + 'a>(reader: R) -> Processor<'a, Parser<Packet<'a>>, R> {
    Processor::new(reader, 4096)
} 