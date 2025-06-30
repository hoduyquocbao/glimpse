use glimpse::{Parser, Packet, Readable};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use memmap2::Mmap;
use rayon::prelude::*;

fn parse(path: &Path) -> io::Result<()> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let data = &mmap[..];
    let chunk_size = 100_000; // mỗi chunk ~100kB (tùy chỉnh theo thực tế)
    let mut offsets = vec![0];
    let mut cursor = 0;
    // Xác định các offset bắt đầu chunk, đảm bảo không cắt packet
    while cursor < data.len() {
        let mut pos = cursor + chunk_size;
        if pos >= data.len() {
            break;
        }
        // Lùi lại cho đến khi tìm được ranh giới packet hợp lệ
        let mut found = false;
        for back in 0..16 {
            if pos < 4 + back { break; }
            let try_pos = pos - back;
            // Kiểm tra xem có thể parse header ở vị trí này không
            if let Ok((header, _)) = Parser::<Packet>::read(&data[try_pos..]) {
                // Nếu parse được header, kiểm tra xem payload có đủ không
                let size = header.header.length as usize;
                if try_pos + 4 + size <= data.len() {
                    pos = try_pos;
                    found = true;
                    break;
                }
            }
        }
        if !found { pos = cursor + chunk_size; } // fallback: chia cứng
        offsets.push(pos);
        cursor = pos;
    }
    offsets.push(data.len());
    // Xử lý song song từng chunk
    let windows: Vec<_> = offsets.windows(2).collect();
    let results: Vec<(usize, usize)> = windows.into_par_iter().map(|w| {
        let start = w[0];
        let end = w[1];
        let mut cursor = &data[start..end];
        let mut count = 0;
        let mut total = 0;
        while !cursor.is_empty() {
            match Parser::<Packet>::read(cursor) {
                Ok((packet, rest)) => {
                    count += 1;
                    total += packet.payload.len();
                    cursor = rest;
                }
                Err(_) => break,
            }
        }
        (count, total)
    }).collect();
    let (count, total) = results.into_iter().fold((0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
    println!("[mmap-parallel] Parsed {} packets, total payload length: {}", count, total);
    Ok(())
}

fn main() -> io::Result<()> {
    // Tạo file giả lập lớn (nhiều packet)
    let path = Path::new("big_packets.bin");
    if !path.exists() {
        let mut file = File::create(path)?;
        for i in 0..10_000 {
            let version = (i % 10 + 1) as u16;
            let payload = vec![b'a' + (i % 26) as u8; 100];
            file.write_all(&version.to_be_bytes())?;
            file.write_all(&(payload.len() as u16).to_be_bytes())?;
            file.write_all(&payload)?;
        }
    }
    parse(path)?;
    Ok(())
} 