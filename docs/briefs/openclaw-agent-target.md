---
title: Đóng việc chọn đúng agent OpenClaw nhận tin, đã chạy ổn
audience: internal
size: S
ask: test
---

**Là gì:** Trước ngày 17/10 WaggleDance chỉ gửi tin tới agent mặc định của OpenClaw. Việc này thêm một cài đặt tên agent nhận tin, và lệnh cài đặt OpenClaw chọn đúng thư mục của agent đó. Cài đặt này vẫn đang được dùng: luồng mới đánh thức wd theo đúng tên agent này.

**Vì sao:** wd là một agent riêng trên OpenClaw; không chọn được thì tin đi nhầm chỗ hoặc không tới.

**Nguồn gốc:** Phát sinh khi nối agent giám sát với wd ngày 17/10; gộp vào bản chính ngày 18/10.

**Chi phí và rủi ro:** Nhỏ, một bước việc, các bài kiểm tra tự động đều qua. Anh không cần kiểm tra riêng: wd còn nhận tin trên Telegram nghĩa là cài đặt này chạy đúng.

**Anh cần làm:** (a) Đồng ý đóng hồ sơ (b) Giữ mở.
**Đề xuất:** (a) — đã chạy hằng ngày trong luồng wd.

**Chuẩn xong:**
- Hồ sơ không còn trên bảng việc
- wd tiếp tục nhận tin từ WaggleDance trên Telegram
