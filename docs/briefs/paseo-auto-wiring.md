---
title: Mọi workspace Paseo tự thành dự án, không cần khai báo tay
audience: owner
size: M
ask: test
---

**Là gì:** Trước đây mỗi workspace Paseo phải được thêm tay vào WaggleDance, và vài dự án như happyfarm bị thiếu. Nay WaggleDance tự tìm Paseo trên máy, tự đăng ký mọi workspace thành dự án, tự chọn workspace cho leader khi một dự án có nhiều cái, và đọc cài đặt riêng từng dự án từ một tệp nằm ngay trong repo. Lệnh cài đặt Paseo hết lỗi, leader mới mở kèm model mặc định.

**Vì sao:** Anh nói thêm tay từng workspace là sai, vì Paseo đã biết rất nhiều workspace.

**Nguồn gốc:** Anh yêu cầu ngày 14/10; gộp ngày 15/10. Quy tắc "mặc định không nhận việc" của đợt này đã đổi thành mặc định nhận ngày 23/10.

**Chi phí và rủi ro:** Vừa, đã xong. Rủi ro: workspace thử nghiệm trong Paseo cũng thành dự án trên bảng.

**Anh cần làm:** (a) Kiểm tra rồi chấp nhận (b) Báo lỗi.
**Đề xuất:** (a) — anh đang dùng nó hằng ngày.

**Chuẩn xong:**
- Mở bảng: mọi workspace có trong Paseo đều có dự án tương ứng
- Tạo một workspace mới trong Paseo: nó hiện thành dự án mà không cần khai báo
- Dự án có hai workspace vẫn giao việc được, không bị từ chối
