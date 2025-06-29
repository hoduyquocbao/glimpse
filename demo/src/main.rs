use glimpse::{PacketSchema, Processor};
use std::io;

fn main() {
    // Buffer gốc chứa 3 gói tin nối tiếp nhau.
    let data: &[u8] = &[
        0, 1, 0, 5, b'h', b'e', b'l', b'l', b'o',
        0, 2, 0, 5, b'w', b'o', b'r', b'l', b'd',
        0, 3, 0, 7, b'g', b'l', b'i', b'm', b'p', b's', b'e',
    ];
    let source = io::Cursor::new(data);
    let processor = Processor::<PacketSchema, _>::new(source, 12);
    // Sử dụng Fluent API: filter và map
    let results: Vec<usize> = processor
        .filter(|pkt| pkt.header.version > 1)
        .map(|pkt| pkt.payload.len())
        .collect();
    println!("Payload lengths for packets with version > 1: {:?}", results);
    assert_eq!(results, vec![5, 7]);
} 