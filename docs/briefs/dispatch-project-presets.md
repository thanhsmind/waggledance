---
title: Agent giao việc được mọi loại agent mà dự án tự khai báo
audience: internal
size: S
ask: test
---

**Là gì:** Trước đây nút Start trên bảng khởi động được các loại agent dự án khai báo, còn agent giao việc qua công cụ chỉ tra danh sách chung của máy, vốn trống, nên luôn bị từ chối. Nay tên agent được tra danh sách chung trước, rồi tới khai báo của dự án.

**Vì sao:** Người bấm nút làm được còn agent điều phối thì không, trên cùng một máy.

**Nguồn gốc:** Ghi lại ngày 25/08/2026 khi khép đợt cho agent điều phối thấy danh sách agent của dự án.

**Chi phí và rủi ro:** Nhỏ, hai bước, đã vào nhánh chính. Không cấp thêm quyền: người gọi chỉ được đưa tên, không bao giờ đưa dòng lệnh. Tên trùng ở hai nơi vẫn theo danh sách chung như trước.

**Anh cần làm:** (a) Đồng ý đóng (b) Yêu cầu thử lại trước khi đóng. **Đề xuất:** (a) Đồng ý đóng — việc nội bộ đã chạy, còn lại chỉ là dọn nhánh cũ.

**Chuẩn xong:**
- Giao việc bằng tên agent dự án khai báo thì agent khởi động
- Tên không có ở đâu thì bị từ chối, lời từ chối nêu tên dự án
- Việc này rời khỏi danh sách việc đang mở
