---
title: Nút Start trên bảng khởi động agent đúng như dự án đã khai báo
audience: internal
size: S
ask: test
---

**Là gì:** Trước đây nút Start trên bảng chỉ lấy dòng lệnh của agent, bỏ qua biến môi trường và bước tin cậy thư mục mà dự án khai báo kèm. Nay bảng đọc trọn khai báo, nên agent khởi động từ bảng hay từ lệnh giao việc đều giống nhau.

**Vì sao:** Hai nơi hiểu một khai báo khác nhau thì agent có thể treo ở màn hỏi tin cậy khi bấm từ bảng.

**Nguồn gốc:** Phần còn lại của đợt đọc trọn khai báo agent ngày 25/08/2026.

**Chi phí và rủi ro:** Nhỏ, một bước, đã vào nhánh chính và quy trình đã đóng. Chưa bấm Start thật vì sẽ mở agent thật trên máy anh; phần nền đã thử thật ở đợt trước. Còn một chỗ hở: bước tin cậy thất bại thì thẻ chưa báo.

**Anh cần làm:** (a) Đồng ý đóng (b) Yêu cầu thử thật trước khi đóng. **Đề xuất:** (a) Đồng ý đóng — việc nội bộ, đã chạy hai tháng, anh không cần thao tác gì.

**Chuẩn xong:**
- Bấm Start cho dự án có khai báo tin cậy thì agent không dừng ở màn hỏi tin cậy
- Dự án khai báo đơn giản vẫn khởi động như cũ
- Việc này rời khỏi danh sách việc đang mở
