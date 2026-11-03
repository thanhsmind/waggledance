---
title: Đóng việc xếp logo trước trạng thái, vì các chỉnh sau đã thay thế
audience: owner
size: S
ask: drop
---

**Là gì:** Việc này thống nhất thứ tự đọc trên mọi chỗ hiện terminal: logo agent trước, rồi trạng thái, rồi tiêu đề. Đã vào nhánh chính ngày 2026-08-25.

**Vì sao:** Huy hiệu và tab khung từng đặt trạng thái trước logo, còn danh sách terminal thì ngược lại, nên ba chỗ không khớp nhau.

**Nguồn gốc:** Một trong loạt chỉnh logo agent anh yêu cầu ngày 2026-08-25. Cùng ngày, hai việc sau đã viết lại các chỗ đó: huy hiệu bỏ hẳn chữ trạng thái và để logo mang màu trạng thái, rồi chấm màu trạng thái bị gỡ khỏi mọi terminal. Không có hồ sơ thiết kế riêng, nguồn là lịch sử commit và quyết định đã ghi.

**Chi phí và rủi ro:** Không tốn thêm gì. Thứ tự logo trước vẫn còn trên tab và danh sách terminal; đóng việc này không gỡ gì.

**Anh cần làm:** (a) Đồng ý đóng vì các chỉnh sau đã thay thế (b) Giữ lại để kiểm tra riêng. **Đề xuất:** (a) — những gì anh thấy hôm nay được kiểm tra qua việc logo mang màu trạng thái.

**Chuẩn xong:**
- Việc được đóng, ghi rõ các việc nào đã thay thế nó
- Tab khung và danh sách terminal vẫn đọc logo trước, trạng thái sau
