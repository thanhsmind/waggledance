---
title: Việc bấm chạy từ bảng tự đóng khung terminal khi agent báo xong
audience: owner
size: M
ask: test
---

**Là gì:** Trước đây việc bấm chạy từ bảng xong rồi vẫn ghi "đang làm" mãi, khung terminal vẫn mở. Giờ một vòng quét nền thấy agent báo xong thì lưu hội thoại, đánh dấu xong và đóng khung; khung đã biến mất thì ghi "mất".

**Vì sao:** Khung bỏ quên chiếm máy và làm bảng sai, anh tưởng việc còn chạy. Ngày 31/8 còn thấy ba việc từ 25/8 kẹt "đang làm".

**Nguồn gốc:** Từ mục yêu cầu cùng tên trong danh sách việc, ghi ngày 31/8 sau khi một lượt tổng kết bấm từ bảng xong mà không đóng. Anh trả lời ba câu hỏi phạm vi cùng ngày; đã gộp vào bản chính. Mục yêu cầu vẫn treo ở trạng thái đề xuất.

**Chi phí và rủi ro:** Vừa, đã xong. Chỉ đóng khung khi chính agent báo xong; không thêm thông báo nào.

**Anh cần làm:** (a) Thử rồi đóng cả việc lẫn mục yêu cầu (b) Báo lỗi (c) Tắt vòng quét. **Đề xuất:** (a) — vòng quét đang chạy ổn từ tháng tám.

**Chuẩn xong:**
- Bấm chạy một việc từ bảng, agent báo xong thì khung tự đóng sau vài phút
- Việc đó trên bảng chuyển sang xong
- Không còn việc nào ghi "đang làm" khi khung đã mất
