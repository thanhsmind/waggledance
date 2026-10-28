---
title: Mọi dự án nhận việc giao theo mặc định, tắt bằng một dòng trong repo
audience: owner
size: M
ask: test
---

**Là gì:** Trước đây dự án tự đăng ký từ Paseo từ chối việc được giao nếu repo thiếu tệp cài đặt bật nhận việc. Nay mọi dự án được quản lý đều nhận việc theo mặc định; muốn tắt, thêm một dòng vào tệp cài đặt trong repo đó. Trang Settings bỏ ô tích theo dự án, thay bằng trạng thái thật của từng dự án kèm cách tắt.

**Vì sao:** Ngày 23/10 một việc giao cho dự án blog bị từ chối chỉ vì thiếu tệp đó.

**Nguồn gốc:** Anh yêu cầu ngày 23/10; thay quy tắc mặc định tắt của đợt tự nối Paseo ngày 14/10. Đã gộp cùng ngày.

**Chi phí và rủi ro:** Vừa, đã xong. Dự án anh không muốn agent động vào giờ phải tắt tay.

**Anh cần làm:** (a) Kiểm tra rồi chấp nhận (b) Báo lỗi (c) Quay lại mặc định tắt.
**Đề xuất:** (a) — đúng như anh đã yêu cầu.

**Chuẩn xong:**
- Giao việc cho dự án chưa có tệp cài đặt: việc được nhận
- Trang Settings, mục Orchestration, liệt kê từng dự án bật hay tắt
- Dự án đã tắt từ chối việc và chỉ rõ dòng cần xoá
- Nút duyệt và nút chạy trên bảng khớp trạng thái đó
