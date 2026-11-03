---
title: Kiểm tra cú pháp bộ cài Windows trong CI — đã thêm, chờ đóng
audience: internal
size: S
ask: test
---

**Là gì:** Bộ cài cho Windows được viết lại khi đổi tên sản phẩm thành WaggleDance nhưng chưa ai chạy thử, vì máy làm việc không có PowerShell. Ngày 23/8 một bước đã được thêm vào CI: trên máy Windows của GitHub, nó đọc toàn bộ bộ cài và báo lỗi nếu sai cú pháp.

**Vì sao:** Một lỗi cú pháp trong bộ cài nghĩa là người dùng Windows không cài được, và hiện không ai thấy điều đó trước khi phát hành.

**Nguồn gốc:** Ghi nhận khi đổi tên sản phẩm giữa tháng 8; bước kiểm tra đã vào nhánh chính nhưng mục chưa được đóng.

**Chi phí và rủi ro:** Không tốn thêm. Giới hạn cần biết: dự án là bản fork nên GitHub Actions chưa từng thực sự chạy, tức bước này có sẵn nhưng chưa chạy lần nào. Lần phát hành có dựng bản Windows mới là lần chứng minh đầu tiên.

**Anh cần làm:** (a) Đồng ý đóng mục này (b) Giữ mở cho tới khi CI chạy được một lần. **Đề xuất:** (a), vì bước kiểm tra đã nằm trong CI và không cần anh thao tác gì.

**Chuẩn xong:**
- Mục này được đóng trên bảng công việc
- Lần chạy CI đầu tiên trên Windows báo bộ cài đọc được
