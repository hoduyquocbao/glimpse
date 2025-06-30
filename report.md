# Báo cáo: Thiết kế & Triển khai Ban đầu Module `insight`

**ID Nhiệm vụ Gốc:** `T018`

## 1. Tổng quan

Báo cáo này tổng kết các hoạt động trong giai đoạn đầu của **Giai đoạn 3: Ứng dụng**, tập trung vào việc nghiên cứu, thiết kế, và triển khai bộ khung cho module `insight` - một engine phân tích log thời gian thực.

Mục tiêu là xây dựng một nền tảng vững chắc cho `insight` dựa trên các nguyên tắc hiệu năng và zero-copy đã được chứng minh trong `glimpse` và `storage`.

## 2. Kết quả Thiết kế & Kiến trúc

Các quyết định kiến trúc quan trọng đã được đưa ra và ghi lại trong `insight/architecture.csv`:

*   **Định dạng Log Ưu tiên:** Quyết định tập trung vào định dạng **JSON-lines** (`JSON First`). Định dạng này có cấu trúc rõ ràng, cho phép phân tích hiệu năng cao mà không cần đến các biểu thức chính quy (regex) tốn kém.
*   **Mô hình Phân tích Lười biếng (Lazy Parsing):**
    *   `Parser` chỉ có trách nhiệm xác định ranh giới của một bản ghi log JSON hoàn chỉnh (một đối tượng `{...}`).
    *   `Entry` (Lens) là một cấu trúc zero-copy chỉ chứa tham chiếu đến slice byte của bản ghi. Việc truy vấn các trường cụ thể (`Entry::text()`) được thực hiện "theo yêu cầu" thay vì phân tích toàn bộ bản ghi ngay từ đầu.
*   **API Công khai:** Thiết kế một API theo mẫu **Fluent Iterator** (`insight::open(...)`), cho phép người dùng xử lý file log một cách tự nhiên và hiệu quả, tương tự như các iterator chuẩn của Rust.

## 3. Kết quả Triển khai

Các nhiệm vụ triển khai (`T019`, `T020`, `T021`) đã được hoàn thành:

*   Một bộ khung hoàn chỉnh cho module `insight` đã được xây dựng và tích hợp vào workspace.
*   `Parser` và `Entry` đã được triển khai, cùng với API `insight::open`.
*   Một ví dụ thực tế (`examples/main.rs`) đã được tạo để chứng minh toàn bộ luồng hoạt động: mở file, lọc các bản ghi log "error", và in ra thông điệp. Ví dụ đã chạy thành công.

## 4. Nợ kỹ thuật & Quyết định Quan trọng

Trong quá trình triển khai, một vấn đề về lifetime đã phát sinh với thư viện `sonic-rs` được lựa chọn ban đầu. Để đảm bảo tiến độ và có một phiên bản hoạt động, một quyết định kỹ thuật đã được đưa ra:

*   **Quyết định:** Tạm thời sử dụng thư viện `serde_json` để thực hiện việc truy vấn.
*   **Hệ quả:** Hàm `Entry::text()` hiện tại trả về một `String` (được cấp phát bộ nhớ mới) thay vì một `&str` (tham chiếu zero-copy).
*   **Ghi nhận:** Đây là một **nợ kỹ thuật có chủ đích** về hiệu năng. Nó đã được ghi lại chính thức trong `insight/todo.csv` với ID **`T022`** và trong `insight/memories.csv` với ID **`M001`**.

## 5. Kết luận & Bước tiếp theo

Giai đoạn thiết kế và triển khai ban đầu đã thành công. Chúng ta đã có một phiên bản `insight` hoạt động, chứng minh được tính đúng đắn của kiến trúc đã đề ra.

Ưu tiên hàng đầu tiếp theo là giải quyết nợ kỹ thuật trong **`T022`** để khôi phục triết lý zero-copy và đảm bảo hiệu năng tối đa cho module.

---
Hệ thống đã sẵn sàng cho chỉ đạo tiếp theo.

# Báo cáo: Giải quyết Nợ kỹ thuật Module `insight`

**ID Nhiệm vụ:** `T022`

## 1. Bối cảnh & Mục tiêu

Báo cáo này trình bày kết quả của nhiệm vụ `T022`, được kích hoạt để giải quyết một khoản nợ kỹ thuật quan trọng về hiệu năng trong module `insight`.

Mục tiêu là tái cấu trúc hàm `insight::Entry::text` để tuân thủ nghiêm ngặt triết lý zero-copy của hệ thống, bằng cách thay đổi chữ ký từ `-> Option<String>` (cấp phát bộ nhớ) thành `-> Option<&'a str>` (tham chiếu zero-copy).

## 2. Quá trình & Quyết định

Quá trình giải quyết nợ kỹ thuật đã trải qua nhiều lần thử nghiệm và đánh giá:

1.  **Thử nghiệm Thư viện Zero-Copy:** Các thư viện như `serde_json_borrow` đã được nghiên cứu và tích hợp thử. Tuy nhiên, chúng đã gây ra các vấn đề phức tạp về API, dependency, và lifetime mà không dễ dàng tương thích với kiến trúc hiện tại.

2.  **Quyết định then chốt:** Thay vì phụ thuộc vào một thư viện bên ngoài phức tạp, quyết định cuối cùng là **tự triển khai một trình phân tích (parser) thủ công, đơn giản** ngay bên trong hàm `Entry::text`.

## 3. Kết quả Triển khai

*   Hàm `Entry::text` đã được viết lại hoàn toàn. Nó thực hiện quét byte trên slice dữ liệu thô để tìm kiếm mẫu `"key": "value"` và trả về một `&str` mượn trực tiếp từ buffer gốc.
*   Giải pháp này đã **loại bỏ hoàn toàn việc cấp phát bộ nhớ** trong quá trình truy vấn, đưa `insight` trở lại đúng với triết lý hiệu năng cao.
*   Toàn bộ các dependency liên quan đến `serde_json` và `serde_json_borrow` đã được loại bỏ khỏi module, giúp giảm sự phức tạp và tăng tính tự chủ của `insight`.
*   Hệ thống đã được xác minh lại bằng unit test và ví dụ thực tế (`examples/main.rs`), cho thấy nó hoạt động chính xác sau khi tái cấu trúc.

## 4. Kết luận

Nợ kỹ thuật về hiệu năng trong `insight` đã được **giải quyết triệt để**. Module này hiện không chỉ đúng về mặt chức năng mà còn mạnh mẽ về mặt hiệu năng, sẵn sàng cho các ứng dụng phân tích log quy mô lớn.

Toàn bộ backlog của `insight` đã được hoàn thành. Hệ thống đã sẵn sàng cho chỉ đạo tiếp theo.
