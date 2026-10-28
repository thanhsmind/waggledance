---
title: Agent điều phối thấy được ô terminal sẵn có để dùng lại thay vì mở mới
audience: internal
size: M
ask: test
---

**Là gì:** Trước đây khi hỏi trạng thái dự án, agent điều phối chỉ biết tình hình công việc bee. Nay câu trả lời có thêm tên các loại agent dự án khai báo và các ô terminal đang có, kèm trạng thái bận hay rảnh, để agent dùng lại ô rảnh thay vì mở mới.

**Vì sao:** Quy tắc của anh: máy chỉ chạy một herdr, có ô sẵn thì dùng ô đó.

**Nguồn gốc:** Từ đợt tìm hiểu cách điều phối nhiều agent ngày 25/08/2026.

**Chi phí và rủi ro:** Vừa, năm bước, đã vào nhánh chính và chạy với herdr thật. Chỉ đọc, không cấp quyền mới, chỉ lộ tên chứ không lộ dòng lệnh. herdr không chạy thì phần còn lại vẫn trả về đủ. Quyết định dùng lại hay mở mới thuộc về agent.

**Anh cần làm:** (a) Đồng ý đóng (b) Yêu cầu thử lại trước khi đóng. **Đề xuất:** (a) Đồng ý đóng — việc nội bộ đã chạy, còn lại chỉ là dọn nhánh cũ.

**Chuẩn xong:**
- Hỏi trạng thái dự án thì thấy tên agent khai báo và các ô đang mở
- Tắt herdr thì câu trả lời vẫn có, kèm lý do thiếu danh sách ô
- Việc này rời khỏi danh sách việc đang mở
