---
title: Tab Usage cho thấy mỗi dự án và model đã tiêu bao nhiêu token
audience: owner
size: L
ask: test
---

**Là gì:** Trước đây không có chỗ nào xem agent đã tiêu bao nhiêu token. Nay trang chủ WaggleDance có tab Usage: token theo dự án và theo model (phiên chính, subagent tách riêng) trong 1, 7, 14 hoặc 30 ngày, kèm danh sách từng lượt agent chạy và các tính năng đã xong. Số đọc thẳng từ nhật ký Claude Code, pi và Codex nên việc đang dở cũng hiện.

**Vì sao:** Anh muốn biết mỗi model subagent dùng bao nhiêu token để chọn model hợp với từng việc.

**Nguồn gốc:** Anh yêu cầu ngày 23/10; sau lần thử đầu anh góp ý đổi nguồn số và thêm danh sách lượt chạy. Đã gộp cùng ngày.

**Chi phí và rủi ro:** Lớn, nhiều đợt sửa, đã xong. Chỉ tính token, không quy ra tiền. Nhật ký lớn có thể làm lần mở đầu chậm.

**Anh cần làm:** (a) Kiểm tra rồi chấp nhận (b) Báo chỗ sai.
**Đề xuất:** (a) — anh đã thử một vòng, chỉ còn xác nhận.

**Chuẩn xong:**
- Tab Usage mở được, đổi khung ngày thì số đổi theo
- Các cột theo ngày cộng lại bằng tổng
- Danh sách lượt chạy hiện tên việc, model, dự án, thời gian và token
