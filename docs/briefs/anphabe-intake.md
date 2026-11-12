---
title: Supervisor Haiku nhặt việc code từ anphabe, giao cho leader từng project
audience: owner
size: M
ask: build
---

**Là gì:** Cứ 15 phút, waggledance chạy một lượt Claude Haiku 5.5 đọc Visual Feedback và task trên anphabe có liên quan tới bạn. Haiku chỉ giữ việc code, bỏ việc digital, rồi đoán project. Mục chắc từ 0,8 trở lên được giao thẳng cho leader của project đó; mục chưa chắc bị giữ lại và báo bạn.

**Vì sao:** Góp ý và task code nằm rải trên anphabe, bạn phải tự đọc rồi tự chuyển sang từng repo. Vòng này làm thay bước đó.

**Nguồn gốc:** Yêu cầu ngày 2026-11-12. Lần chạy thử mất 36 giây và 0,012 USD mỗi lượt.

**Chi phí và rủi ro:** Khoảng 1,15 USD mỗi ngày. Rủi ro chính là giao nhầm project; ngưỡng 0,8 giảm rủi ro này. Không ghi gì ngược lên anphabe.

**Anh cần làm:** (a) Đồng ý các mặc định (b) Đổi một mặc định (c) Dừng. **Đề xuất:** (a).

**Chuẩn xong:**
- Lệnh chạy tay in ra mục đã giao, mục bị giữ, mục bỏ qua
- Lượt thứ hai không giao trùng mục nào
- Lệnh xem danh sách hiện các mục bị giữ
- Bật mục intake trong cấu hình thì daemon tự chạy mỗi 15 phút
