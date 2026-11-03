---
title: Chạy thử bộ kích hoạt tự động mà không cần bật nó thật
audience: owner
size: S
ask: build
---

**Là gì:** Trang Settings có ô "Dry run" để xem bộ kích hoạt tự động sẽ làm gì mà không đánh thức agent. Hôm nay ô này chỉ có tác dụng khi ô bật kích hoạt cũng được tích. Sau khi sửa, chỉ tích "Dry run" là bộ kích hoạt chạy để ghi nhật ký, không giao việc.

**Vì sao:** Chạy thử là để xem trước rồi mới bật thật. Bắt bật cả hai khiến chỉ cần quên một ô là agent bị đánh thức thật.

**Nguồn gốc:** Phát hiện khi đóng tính năng kích hoạt tự động, ngày 31/8.

**Chi phí và rủi ro:** Nhỏ: đổi một điều kiện khởi động, thêm một bài kiểm thử. Rủi ro là chạy thử lỡ giao việc thật; bài kiểm thử phải chặn điều đó.

**Anh cần làm:** (a) Duyệt làm (b) Hoãn (c) Bỏ. **Đề xuất:** (a), vì việc nhỏ và làm ô Settings đúng như chữ trên nó.

**Chuẩn xong:**
- Chỉ tích "Dry run" thì nhật ký ghi các lần kích hoạt lẽ ra xảy ra
- Không agent nào bị đánh thức khi chỉ tích "Dry run"
- Tích cả hai rồi bỏ "Dry run" thì kích hoạt thật như hôm nay
