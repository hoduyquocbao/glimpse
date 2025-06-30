# Báo cáo Hiệu năng - Crate `storage`

## Tóm tắt

Báo cáo này phân tích hiệu năng của ba phương pháp xử lý dữ liệu nhị phân trong crate `storage`. Kết quả benchmark cho thấy phương pháp xử lý song song trên vùng nhớ được ánh xạ (`mmap_parallel`) cung cấp hiệu suất vượt trội, trong khi phương pháp xử lý dựa trên stream (`streaming_processor`) có chi phí hiệu năng cao nhất.

## Kết quả Chi tiết

Các phép đo được thực hiện trên một file dữ liệu 100,000 packet.

| Phương pháp thực thi | Thời gian trung bình | Ghi chú |
| :--- | :--- | :--- |
| **`mmap_parallel`** | **~82 µs** | **Nhanh nhất**. Tận dụng xử lý song song. |
| `mmap_sequential` | ~748 µs | Cơ sở để so sánh. Chậm hơn xử lý song song ~9 lần. |
| `streaming_processor` | ~2.19 ms | Chậm nhất, chậm hơn ~27 lần so với song song. |

## Phân tích

1.  **Hiệu quả của Xử lý Song song (`mmap_parallel`):**
    *   Phương pháp này đạt được tốc độ xử lý nhanh hơn khoảng **9.1 lần** so với xử lý tuần tự (`mmap_sequential`).
    *   Điều này khẳng định rằng việc sử dụng Rayon để chia nhỏ và xử lý các khối dữ liệu trên một `mmap` là một chiến lược cực kỳ hiệu quả cho các tác vụ có thể song song hóa như thế này.

2.  **Chi phí của Stream Processor (`streaming_processor`):**
    *   Phương pháp này chậm hơn đáng kể vì chi phí tạo và quản lý trạng thái của một iterator (`stream processor`).
    *   Trong khi iterator cung cấp một giao diện trừu tượng và an toàn, nó không phù hợp cho các tác vụ yêu cầu quét toàn bộ dữ liệu với thông lượng tối đa so với việc truy cập trực tiếp vào bộ nhớ.

## Đề xuất

*   **Ưu tiên sử dụng `storage::batch` (tương ứng `mmap_parallel`)** cho các tác vụ cần xử lý toàn bộ dữ liệu với hiệu suất cao nhất.
*   Cân nhắc sử dụng `storage::stream` (`streaming_processor`) chỉ khi:
    *   Logic xử lý cho mỗi packet phức tạp và không dễ song song hóa.
    *   Chỉ cần xử lý một phần của dữ liệu (ví dụ: lấy N packet đầu tiên).
    *   Cần một giao diện lập trình an toàn và trừu tượng hơn, chấp nhận đánh đổi hiệu năng. 