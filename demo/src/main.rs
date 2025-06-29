use glimpse::{PacketSchema, Readable};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use memmap2::Mmap;

fn parse_mmap(path: &Path) -> io::Result<()> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let mut cursor = &mmap[..];
    let mut count = 0;
    let mut total_payload = 0;
    while !cursor.is_empty() {
        match PacketSchema::read(cursor) {
            Ok((packet, rest)) => {
                count += 1;
                total_payload += packet.payload.len();
                cursor = rest;
            }
            Err(_) => break,
        }
    }
    println!("[mmap] Parsed {} packets, total payload length: {}", count, total_payload);
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
    parse_mmap(path)?;
    Ok(())
} 