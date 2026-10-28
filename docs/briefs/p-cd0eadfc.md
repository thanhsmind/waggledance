---
title: Bỏ mục lỗi lệnh thêm việc tồn đọng của bee — lệnh đã chạy đúng
audience: internal
size: S
ask: drop
---

**Là gì:** Mục này ban đầu nói lệnh thêm một việc tồn đọng của bee từ chối mọi cách gọi. Kiểm tra lại ngày 25/8 cho thấy lệnh chạy đúng khi đủ bốn thông tin bắt buộc; điều còn lại chỉ là thông báo từ chối không nói thiếu thông tin nào. Hôm nay bee đã nêu rõ từng thông tin còn thiếu và cách sửa.

**Vì sao:** Thông báo mơ hồ làm agent đoán mò cách gọi lệnh. Người gặp là agent ghi việc tồn đọng, không phải anh.

**Nguồn gốc:** Ghi nhận khi một agent gọi lệnh thiếu thông tin, thu hẹp ngày 25/8 thành chuyện câu thông báo, và chính mục ghi "thu hẹp hoặc bỏ".

**Chi phí và rủi ro:** Không tốn gì để đóng. Lệnh này thuộc bee, không thuộc mã của WaggleDance, nên nếu còn lỗi thì phải báo lên bee chứ không sửa ở đây.

**Anh cần làm:** (a) Bỏ mục này (b) Giữ lại. **Đề xuất:** (a), vì lỗi gốc không tồn tại và câu thông báo nay đã nêu đúng thông tin thiếu.

**Chuẩn xong:**
- Mục này được đóng với lý do lệnh đã chạy đúng
- Gọi lệnh thiếu thông tin thì bee nêu tên thông tin còn thiếu
