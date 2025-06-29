use glimpse::{PacketSchema, Fault, Packet, Processor};
use std::io;

fn main() {
    // Buffer gốc chứa 3 gói tin nối tiếp nhau.
    let data: &[u8] = &[
        0, 1, 0, 5, b'h', b'e', b'l', b'l', b'o',
        0, 2, 0, 5, b'w', b'o', b'r', b'l', b'd',
        0, 3, 0, 7, b'g', b'l', b'i', b'm', b'p', b's', b'e',
    ];
    // Giả lập một source có thể đọc (ví dụ: file hoặc network stream)
    let source = io::Cursor::new(data);
    // Tạo processor với một cửa sổ nhỏ (12 bytes) để kiểm tra boundary.
    let mut processor = Processor::<PacketSchema, _>::new(source, 12);
    let mut count = 0;
    while let Ok(Some(packet)) = processor.next() {
        count += 1;
        println!("Read Packet {}: {:?}", count, packet);
        // Xác thực
        match count {
            1 => assert_eq!(packet.payload, b"hello"),
            2 => assert_eq!(packet.payload, b"world"),
            3 => assert_eq!(packet.payload, b"glimpse"),
            _ => panic!("Should not have more packets"),
        }
    }
    println!("\nFinished processing {} packets.", count);
    assert_eq!(count, 3);
} 