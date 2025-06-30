# Báo cáo Hiệu năng Module `storage`

**ID Nhiệm vụ:** `T015`

## 1. Tổng quan

Báo cáo này trình bày kết quả đo lường hiệu năng của ba hàm cốt lõi trong module `storage`: `scan`, `batch`, và `stream`. Mục tiêu là để hiểu rõ và so sánh tốc độ xử lý của từng phương pháp trên một tập dữ liệu lớn (100,000 packets).

## 2. Kết quả Benchmark

Các phép đo được thực hiện bằng `criterion` trên một bộ dữ liệu giả lập trong bộ nhớ.

| Hàm | Thời gian thực thi (Trung bình) |
| :--- | :--- |
| `storage::scan` | ~7.7 ns |
| `storage::batch` | ~301 µs |
| `storage::stream`| ~2.86 ms |

## 3. Phân tích

Kết quả cho thấy sự khác biệt rõ rệt về hiệu năng, phản ánh đúng thiết kế và mục đích sử dụng của từng hàm:

*   **`scan` (~7.7 ns/op):** Cực kỳ nhanh. Đây là chi phí cơ bản để phân tích cú pháp **một packet duy nhất** từ một buffer đã có sẵn. Tốc độ này là nền tảng cho hiệu năng của các hàm cấp cao hơn.

*   **`batch` (~301 µs/100k packets):** Rất hiệu quả cho việc xử lý toàn bộ một khối dữ liệu lớn trong bộ nhớ. Hàm này tận dụng `rayon` để phân tích các chunk dữ liệu một cách song song trên nhiều lõi CPU, dẫn đến thông lượng (throughput) rất cao. Đây là lựa chọn tối ưu khi toàn bộ dữ liệu đã nằm trong RAM (ví dụ: từ một file đã được `mmap`).

*   **`stream` (~2.86 ms/100k packets):** Chậm hơn đáng kể so với `batch`, nhưng đây là điều được dự kiến. Hàm này được thiết kế để hoạt động với các nguồn dữ liệu có tính tuần tự (streaming I/O) như đọc từ file hoặc network socket, nơi dữ liệu đến không liên tục. Chi phí phụ trội (overhead) đến từ việc quản lý buffer, xử lý các trường hợp dữ liệu bị ngắt quãng (boundaries), và cơ chế iterator. Sự đánh đổi này mang lại sự linh hoạt và khả năng xử lý các file lớn hơn bộ nhớ RAM.

## 4. Kết luận

Hệ thống `storage` đã chứng minh được hiệu năng tốt và có thể dự đoán được. Mỗi hàm đều thể hiện sự vượt trội trong kịch bản sử dụng được thiết kế cho nó. Nhiệm vụ `T015` đã hoàn thành thành công.
