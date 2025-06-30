//! # Insight - Real-time Log Analysis Engine
//!
//! `insight` là một engine phân tích log thời gian thực, hiệu năng cao,
//! được xây dựng trên nền tảng zero-copy của `glimpse` và `storage`.

use glimpse::{Fault, Readable};
use std::fs::File;
use std::io;
use std::path::Path;

/// Đại diện cho một mục log JSON (một dòng).
/// Đây là một 'Lens' zero-copy, chỉ giữ một tham chiếu đến dữ liệu gốc.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    /// Slice byte chứa toàn bộ dòng log JSON.
    raw: &'a [u8],
}

impl<'a> Entry<'a> {
    /// Tạo một `Entry` mới từ một slice byte.
    pub fn new(raw: &'a [u8]) -> Self {
        Self { raw }
    }

    /// Tìm một giá trị dạng &str bằng cách quét byte thủ công (zero-copy).
    ///
    /// LƯU Ý: Đây là một parser rất đơn giản, giả định key và value
    /// là các chuỗi trích dẫn không có ký tự escape.
    pub fn text(&self, key: &str) -> Option<&'a str> {
        let key_pattern = format!("\"{}\"", key);
        let key_bytes = key_pattern.as_bytes();

        self.raw
            .windows(key_bytes.len())
            .position(|window| window == key_bytes)
            .and_then(|key_pos| {
                let after_key = &self.raw[key_pos + key_bytes.len()..];
                after_key
                    .iter()
                    .position(|&b| b == b':')
                    .and_then(|colon_pos| {
                        let after_colon = &after_key[colon_pos + 1..];
                        after_colon
                            .iter()
                            .position(|&b| b == b'"')
                            .and_then(|start_quote_pos| {
                                let start_of_value = start_quote_pos + 1;
                                let rest = &after_colon[start_of_value..];
                                rest.iter().position(|&b| b == b'"').map(|end_quote_pos| {
                                    unsafe {
                                        std::str::from_utf8_unchecked(&rest[..end_quote_pos])
                                    }
                                })
                            })
                    })
            })
    }
}

/// Một trình phân tích cú pháp cho các dòng log JSON, tuân thủ `glimpse::Readable`.
/// Đây là một "newtype" wrapper để tránh vi phạm orphan rule.
pub struct Parser;

// Triển khai Readable cho Parser<Entry> để nó có thể được dùng với Processor.
impl<'a> Readable<'a> for Parser {
    type Lens = Entry<'a>;
    type Fault = Fault;

    /// Tìm một mục log JSON hoàn chỉnh trong buffer.
    /// Logic này tìm kiếm một cặp `{...}` cân bằng.
    fn read(source: &'a [u8]) -> Result<(Self::Lens, &'a [u8]), Self::Fault> {
        let mut start = None;
        let mut balance = 0;

        for (i, &byte) in source.iter().enumerate() {
            // Bỏ qua whitespace ở đầu
             if start.is_none() {
                if byte.is_ascii_whitespace() {
                    continue;
                }
                if byte == b'{' {
                    start = Some(i);
                } else {
                    // Ký tự không mong muốn trước khi JSON object bắt đầu
                    return Err(Fault::Invalid);
                }
            }

            if let Some(start_index) = start {
                 match byte {
                    b'{' => {
                        balance += 1;
                    }
                    b'}' => {
                        balance -= 1;
                        if balance == 0 {
                            let end_index = i + 1;
                            let entry = Entry::new(&source[start_index..end_index]);
                            let rest = &source[end_index..];
                            return Ok((entry, rest));
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Nếu duyệt hết buffer mà không tìm thấy cặp ngoặc hoàn chỉnh
        Err(Fault::Underflow)
    }
}

/// Mở một file log và trả về một stream các `Entry` có thể lặp qua.
///
/// # Arguments
///
/// * `path` - Đường dẫn đến file log.
/// * `capacity` - Kích thước của buffer đọc (window).
///
/// # Returns
///
/// Một `Result` chứa `Processor` hoặc một `io::Error`.
pub fn open<'a, P: AsRef<Path>>(
    path: P,
    capacity: usize,
) -> io::Result<glimpse::Processor<'a, Parser, File>> {
    let source = File::open(path)?;
    Ok(glimpse::Processor::new(source, capacity))
}


#[cfg(test)]
mod tests {
    use super::*;
    use glimpse::Readable;

    #[test]
    fn it_parses_and_queries_entry() {
        let data = br#"
            {"timestamp": "2024-07-25T10:00:00Z", "level": "info", "message": "Request processed"}
            {"timestamp": "2024-07-25T10:00:01Z", "level": "warn", "message": "High latency"}
        "#;

        // Bỏ qua whitespace ở đầu
        let trimmed_data = &data[data.iter().position(|&c| !c.is_ascii_whitespace()).unwrap_or(0)..];
        
        // Entry đầu tiên
        let (entry1, rest1) = Parser::read(trimmed_data).unwrap();
        assert_eq!(entry1.text("level"), Some("info"));
        assert_eq!(entry1.text("message"), Some("Request processed"));

        // Entry thứ hai
        let trimmed_rest = &rest1[rest1.iter().position(|&c| !c.is_ascii_whitespace()).unwrap_or(0)..];
        let (entry2, _) = Parser::read(trimmed_rest).unwrap();
        assert_eq!(entry2.text("level"), Some("warn"));
        assert_eq!(entry2.text("message"), Some("High latency"));
    }
} 