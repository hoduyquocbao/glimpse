use insight::open;

fn main() {
    println!("Analyzing log.json...");

    // Mở file log. Sử dụng unwrap cho ví dụ đơn giản.
    let stream = open("log.json", 4096).expect("Failed to open log.json");

    // Lọc ra các log có level là "error" và in message của chúng
    stream
        .filter(|entry| entry.text("level") == Some("error"))
        .for_each(|entry| {
            if let Some(message) = entry.text("message") {
                println!("[ERROR] {}", message);
            }
        });

    println!("Analysis complete.");
} 