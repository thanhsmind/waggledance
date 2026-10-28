---
title: Bỏ bản sửa nhãn ô agent mới giao việc, đã cũ hai tháng
audience: owner
size: S
ask: drop
---

**Là gì:** Khi WaggleDance giao việc cho agent trong một ô terminal, vài giây đầu herdr vẫn coi ô đó là dòng lệnh thường, nên danh sách agent không hiện nó và trang terminal gắn nhãn sai. Bản sửa viết ngày 31/8 coi ô đó là agent ngay từ đầu, nhưng chưa từng được duyệt hay gộp.

**Vì sao:** Anh giao việc xong mà không thấy agent trong danh sách, dễ tưởng việc giao bị hỏng.

**Nguồn gốc:** Agent phát hiện khi chạy thử luồng giao việc ngày 31/8; hồ sơ dừng ở bước tìm hiểu, chưa có quyết định hay kế hoạch nào.

**Chi phí và rủi ro:** Bản sửa nằm riêng một nhánh, đã lệch nhánh chính hai tháng. Từ tháng 10, việc giao cho agent mặc định đi qua Paseo thay vì ô terminal, nên lỗi này hiếm gặp. Gộp bây giờ gần như phải làm và kiểm tra lại từ đầu.

**Anh cần làm:** (a) Bỏ và xóa nhánh cũ (b) Hoãn (c) Duyệt làm lại. **Đề xuất:** (a) — luồng giao việc chính đã chuyển sang Paseo; lỗi tái xuất thì mở việc mới.

**Chuẩn xong:**
- Hồ sơ được đóng với lý do đã được thay thế
- Nhánh cũ được xóa, không còn việc treo
