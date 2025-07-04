// -----------------------------------------------------------------------------
// PHẦN 1: ĐỊNH NGHĨA FRAMEWORK CỐT LÕI
// -----------------------------------------------------------------------------

/// Định nghĩa khả năng đọc một cấu trúc `Lens` zero-copy từ một `source`.
/// Tuân thủ quy tắc "Able Pattern": Readable.
pub trait Readable<'a> {
    /// Cấu trúc "thấu kính" zero-copy, chứa các tham chiếu vào `source`.
    type Lens: 'a;

    /// Kiểu lỗi có thể xảy ra khi đọc.
    type Fault;

    /// Đọc một `Lens` từ `source`, trả về `Lens` và phần `source` còn lại.
    fn read(source: &'a [u8]) -> Result<(Self::Lens, &'a [u8]), Self::Fault>;
}

/// Enum định nghĩa các lỗi có thể xảy ra trong quá trình đọc.
/// Tên "Fault" là một từ đơn, mạnh mẽ thay cho "Error".
#[derive(Debug, PartialEq)]
pub enum Fault {
    /// Buffer đầu vào không đủ dài để đọc.
    Underflow,
    /// Dữ liệu không hợp lệ (ví dụ: độ dài trong header không khớp).
    Invalid,
}

// -----------------------------------------------------------------------------
// PHẦN 2: TRIỂN KHAI CHO MỘT GIAO THỨC CỤ THỂ
// -----------------------------------------------------------------------------

use std::convert::TryInto;
use std::marker::PhantomData;

/// Lens cho Header: chứa các giá trị đã được phân tích.
/// Nó không chứa tham chiếu vì các giá trị (u16) nhỏ và được sao chép.
/// Đây vẫn là một phần của quy trình zero-copy vì chúng ta không cấp phát bộ nhớ mới.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Header {
    pub version: u16,
    pub length: u16,
}

/// A generic, stateless parser definition.
/// The type T is the "Lens" it knows how to read.
pub struct Parser<T>(PhantomData<T>);

impl<'a> Readable<'a> for Parser<Header> {
    type Lens = Header;
    type Fault = Fault;

    fn read(source: &'a [u8]) -> Result<(Self::Lens, &'a [u8]), Self::Fault> {
        // Header yêu cầu 4 bytes (2 cho version, 2 cho length).
        if source.len() < 4 {
            return Err(Fault::Underflow);
        }
        // Tách 4 bytes đầu tiên cho header.
        let (header_bytes, rest) = source.split_at(4);
        // Đổi tên version_bytes -> version, length_bytes -> length
        let version: [u8; 2] = header_bytes[0..2].try_into().unwrap();
        let length: [u8; 2] = header_bytes[2..4].try_into().unwrap();
        let header = Header {
            version: u16::from_be_bytes(version),
            length: u16::from_be_bytes(length),
        };
        Ok((header, rest))
    }
}

/// Lens cho Packet: một cấu trúc zero-copy thực sự.
/// Nó chứa Header (được sao chép vì nhỏ) và một tham chiếu (&[u8])
/// đến payload trong buffer gốc.
#[derive(Debug, PartialEq)]
pub struct Packet<'a> {
    pub header: Header,
    pub payload: &'a [u8],
}

impl<'a> Readable<'a> for Parser<Packet<'a>> {
    type Lens = Packet<'a>;
    type Fault = Fault;

    fn read(source: &'a [u8]) -> Result<(Self::Lens, &'a [u8]), Self::Fault> {
        // Tái sử dụng logic đọc header.
        let (header, after_header) = Parser::<Header>::read(source)?;
        // Đổi tên payload_len -> size
        let size = header.length as usize;
        // Kiểm tra xem buffer còn lại có đủ cho payload không.
        if after_header.len() < size {
            return Err(Fault::Underflow);
        }
        // Tách payload và phần còn lại của buffer.
        let (payload, rest) = after_header.split_at(size);
        let packet = Packet { header, payload };
        Ok((packet, rest))
    }
}

// -----------------------------------------------------------------------------
// PHẦN 4: BỘ XỬ LÝ DÒNG DỮ LIỆU (STREAM PROCESSOR)
// -----------------------------------------------------------------------------

use std::io::{self, Read};

/// Bộ xử lý streaming generic cho bất kỳ parser (impl Readable) và source (impl Read).
/// Quản lý buffer, xử lý window, boundary, tránh OOM khi xử lý file lớn.
pub struct Processor<'a, P, S>
where
    P: Readable<'a, Fault = Fault>,
    S: Read,
{
    parser: std::marker::PhantomData<(&'a (), P)>,
    source: S,
    buffer: Vec<u8>,
    start: usize, // Vị trí bắt đầu dữ liệu hợp lệ
    end: usize,   // Vị trí kết thúc dữ liệu hợp lệ
}

impl<'a, P, S> Processor<'a, P, S>
where
    P: Readable<'a, Fault = Fault>,
    S: Read,
{
    /// Tạo processor mới với buffer (window) kích thước capacity.
    pub fn new(source: S, capacity: usize) -> Self {
        Processor {
            parser: std::marker::PhantomData,
            source,
            buffer: vec![0; capacity],
            start: 0,
            end: 0,
        }
    }

    /// Đọc và trả về Lens tiếp theo từ stream. Trả về Ok(None) khi hết dữ liệu.
    pub fn record(&mut self) -> Result<Option<P::Lens>, io::Error> {
        loop {
            let available = &self.buffer[self.start..self.end];
            if available.is_empty() {
                self.start = 0;
                self.end = self.source.read(&mut self.buffer)?;
                if self.end == 0 {
                    return Ok(None);
                }
                continue;
            }
            // Sử dụng unsafe để mở rộng lifetime (zero-copy, production)
            let data: &'a [u8] = unsafe { std::mem::transmute(available) };
            match P::read(data) {
                Ok((lens, rest)) => {
                    let size = available.len() - rest.len();
                    self.start += size;
                    return Ok(Some(lens));
                }
                Err(Fault::Underflow) => {
                    let size = available.len();
                    self.buffer.copy_within(self.start..self.end, 0);
                    self.start = 0;
                    self.end = size;
                    let count = self.source.read(&mut self.buffer[self.end..])?;
                    if count == 0 {
                        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Incomplete record"));
                    }
                    self.end += count;
                }
                Err(Fault::Invalid) => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid record format"));
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// PHẦN 5: ADAPTER MỞ RỘNG (MAPPER, FILTER, FLUENT API)
// -----------------------------------------------------------------------------

/// Adapter chuyển đổi từng phần tử của một iterator bằng một hàm ánh xạ.
pub struct Mapper<I, F, B>
where
    I: Iterator,
    F: FnMut(I::Item) -> B,
{
    iterator: I,
    function: F,
}

impl<I: Iterator, F, B> Mapper<I, F, B>
where
    F: FnMut(I::Item) -> B,
{
    pub fn new(iterator: I, function: F) -> Self {
        Mapper { iterator, function }
    }
}

impl<I: Iterator, F, B> Iterator for Mapper<I, F, B>
where
    F: FnMut(I::Item) -> B,
{
    type Item = B;
    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(&mut self.function)
    }
}

/// Adapter chỉ giữ lại các phần tử thỏa predicate.
pub struct Filter<I, F> {
    iterator: I,
    predicate: F,
}

impl<I, F> Filter<I, F> {
    pub fn new(iterator: I, predicate: F) -> Self {
        Filter { iterator, predicate }
    }
}

impl<I: Iterator, F> Iterator for Filter<I, F>
where
    F: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.find(|item| (self.predicate)(item))
    }
}

// Biến Processor thành một Iterator
impl<'a, P, S> Iterator for Processor<'a, P, S>
where
    P: Readable<'a, Fault = Fault>,
    S: Read,
{
    type Item = P::Lens;
    fn next(&mut self) -> Option<Self::Item> {
        self.record().ok().flatten()
    }
}

// Fluent API cho Processor
impl<'a, P, S> Processor<'a, P, S>
where
    P: Readable<'a, Fault = Fault>,
    S: Read,
{
    pub fn map<F, B>(self, function: F) -> Mapper<Self, F, B>
    where
        Self: Sized,
        F: FnMut(P::Lens) -> B,
    {
        Mapper::new(self, function)
    }
    pub fn filter<F>(self, predicate: F) -> Filter<Self, F>
    where
        Self: Sized,
        F: FnMut(&P::Lens) -> bool,
    {
        Filter::new(self, predicate)
    }
}
// Fluent API cho Mapper
impl<I: Iterator, F, B> Mapper<I, F, B>
where
    F: FnMut(I::Item) -> B,
{
    pub fn filter<P>(self, predicate: P) -> Filter<Self, P>
    where
        Self: Sized,
        P: FnMut(&B) -> bool,
    {
        Filter::new(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, PartialEq, Clone)]
    struct Frame {
        header: Header,
        payload: Vec<u8>,
    }
    impl<'a> From<Packet<'a>> for Frame {
        fn from(lens: Packet<'a>) -> Self {
            Frame {
                header: lens.header,
                payload: lens.payload.to_vec(),
            }
        }
    }
    #[test]
    fn validity() {
        let buf = [0x01, 0x02, 0x00, 0x05];
        let (header, rest) = Parser::<Header>::read(&buf).unwrap();
        assert_eq!(header.version, 0x0102);
        assert_eq!(header.length, 5);
        assert_eq!(rest.len(), 0);
    }
    #[test]
    fn underflow() {
        let buf = [0x01, 0x02];
        let res = Parser::<Header>::read(&buf);
        assert!(matches!(res, Err(Fault::Underflow)));
    }
    #[test]
    fn integrity() {
        let buf = [0x00, 0x01, 0x00, 0x03, b'a', b'b', b'c'];
        let (packet, rest) = Parser::<Packet>::read(&buf).unwrap();
        assert_eq!(packet.header.version, 1);
        assert_eq!(packet.header.length, 3);
        assert_eq!(packet.payload, b"abc");
        assert_eq!(rest.len(), 0);
    }
    #[test]
    fn empty() {
        let buf = [0x00, 0x01, 0x00, 0x00];
        let (packet, rest) = Parser::<Packet>::read(&buf).unwrap();
        assert_eq!(packet.header.length, 0);
        assert_eq!(packet.payload, b"");
        assert_eq!(rest.len(), 0);
    }
    #[test]
    fn headless() {
        let buf = [0x00, 0x01];
        let res = Parser::<Packet>::read(&buf);
        assert!(matches!(res, Err(Fault::Underflow)));
    }
    #[test]
    fn bodiless() {
        let buf = [0x00, 0x01, 0x00, 0x05, b'a', b'b'];
        let res = Parser::<Packet>::read(&buf);
        assert!(matches!(res, Err(Fault::Underflow)));
    }
    #[test]
    fn boundary() {
        let buf = [0x00, 0x01, 0x00, 0x03, b'a', b'b', b'c', 0x00, 0x02, 0x00, 0x02, b'd', b'e'];
        let source = std::io::Cursor::new(&buf);
        let processor = Processor::<Parser<Packet>, _>::new(source, 7);
        let frames: Vec<Frame> = processor.map(Frame::from).collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].payload, b"abc");
        assert_eq!(frames[1].payload, b"de");
    }
} 