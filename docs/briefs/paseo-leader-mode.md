---
title: Leader mở qua Paseo chạy đúng chế độ, không hỏi quyền từng lệnh
audience: owner
size: M
ask: test
---

**Là gì:** Ngày 22/10 một leader của dự án beehive hiển thị chế độ không hỏi quyền nhưng thực tế vẫn hỏi anh từng lệnh suốt khoảng 9 giờ. Nay WaggleDance luôn mở leader kèm chế độ rõ ràng (mặc định không hỏi quyền), đọc lại chế độ thật, tự chỉnh một lần nếu lệch và báo nếu vẫn lệch, kể cả trước khi giao việc cho leader đã có.

**Vì sao:** Leader kẹt chờ quyền dừng hàng giờ mà không ai biết; Paseo chỉ hiện chế độ cấu hình.

**Nguồn gốc:** Phát hiện khi theo dõi leader beehive ngày 22 và 23/10; đã gộp ngày 23/10, chờ anh kiểm tra.

**Chi phí và rủi ro:** Vừa nhỏ, đã xong. Dự án muốn leader hỏi quyền phải tự đặt chế độ riêng.

**Anh cần làm:** (a) Kiểm tra rồi chấp nhận (b) Báo lỗi.
**Đề xuất:** (a) — sửa đúng lỗi anh đã gặp.

**Chuẩn xong:**
- Giao một việc mới qua wd hoặc trong Paseo: leader làm không dừng hỏi quyền từng lệnh
- Trong Paseo, leader đó chạy đúng chế độ không hỏi quyền
- Nếu vẫn lệch, phản hồi giao việc có một dòng nói rõ
