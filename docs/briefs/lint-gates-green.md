---
title: Hai bước kiểm tra định dạng và chất lượng mã đã xanh trở lại
audience: internal
size: S
ask: test
---

**Là gì:** Cuối tháng 8, hai bước kiểm tra tự động mà dự án tự đặt ra, kiểm tra định dạng mã và kiểm tra chất lượng mã, đều báo đỏ trên nhánh chính, do mã cũ không ai sửa gần đây. Ngày 31/8 agent đã sửa: định dạng lại toàn bộ, xóa vài dòng thừa, và ghi rõ một lý do để bỏ qua một cảnh báo không hợp với cách thiết kế máy chủ.

**Vì sao:** Khi hai bước này đỏ sẵn, không ai phân biệt được lỗi mới với lỗi cũ, và mọi agent sau đều phải làm việc trên nền đỏ.

**Nguồn gốc:** Agent phát hiện ngày 31/8 khi chạy các bước kiểm tra trước lúc gộp; sửa và gộp cùng ngày, hồ sơ chưa đóng.

**Chi phí và rủi ro:** Nhỏ, đã xong. Phần định dạng không đổi logic nào. Anh không thấy gì thay đổi trên giao diện.

**Anh cần làm:** (a) Đồng ý đóng hồ sơ (b) Yêu cầu kiểm tra lại hai bước trên nhánh chính hiện tại trước khi đóng. **Đề xuất:** (a) — việc đã xong từ tháng 8, các lần gộp sau vẫn chạy cùng hai bước này.

**Chuẩn xong:**
- Hồ sơ này được đóng
- Hai bước kiểm tra vẫn xanh trên nhánh chính
