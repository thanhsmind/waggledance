---
title: Lệnh chạy kiểm thử ghi trong cấu hình phải chạy đúng bộ test của dự án
audience: internal
size: S
ask: build
---

**Là gì:** Cấu hình bee của dự án ghi lệnh kiểm thử trỏ vào thư mục của beehive, không có trong WaggleDance, nên chạy là hỏng. Sau khi sửa, lệnh chạy đúng bộ kiểm thử của WaggleDance.

**Vì sao:** Mỗi phiên đều được nhắc lệnh này như lệnh chuẩn, và bước đóng việc dựa vào nó sẽ không chạy gì hoặc báo đỏ giả. Người gặp là mọi agent trong dự án.

**Nguồn gốc:** Phát hiện ngày 30/8 khi rà kế hoạch tính năng hiển thị bảng; quyết định sửa có cùng ngày nhưng chỉ sửa ở nhánh phụ, nhánh chính vẫn giữ giá trị cũ.

**Chi phí và rủi ro:** Rất nhỏ: đổi một dòng cấu hình rồi chạy thử. Chỉ cần chọn lệnh không quá chậm.

**Anh cần làm:** (a) Duyệt làm (b) Hoãn (c) Bỏ. **Đề xuất:** (a), vì lỗi thật, sửa trong vài phút và quyết định đã có sẵn.

**Chuẩn xong:**
- Lệnh kiểm thử trong cấu hình chạy được trên nhánh chính
- Phiên mới thấy đúng lệnh mới
- Bước đóng việc không còn báo đỏ giả
