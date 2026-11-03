---
title: Huy hiệu terminal chỉ còn logo mang màu trạng thái và tiêu đề
audience: owner
size: S
ask: test
---

**Là gì:** Trước đây mỗi huy hiệu terminal nói trạng thái ba lần: logo agent, chấm màu kèm chữ, rồi tiêu đề khung. Nay huy hiệu chỉ còn logo rồi tiêu đề; màu của logo chính là trạng thái, và chữ trạng thái chuyển vào phần hiện khi di chuột, ví dụ "claude — cần duyệt".

**Vì sao:** Huy hiệu gọn hơn, anh đọc tiêu đề việc mà không bị trạng thái lặp lại.

**Nguồn gốc:** Một trong loạt chỉnh logo agent anh yêu cầu ngày 2026-08-25, đã vào nhánh chính cùng ngày, ngay sau việc cho logo mang màu trạng thái. Nguồn chỉ có lịch sử commit.

**Chi phí và rủi ro:** Nhỏ, đã làm xong. Chỉ huy hiệu bỏ chữ trạng thái; tab khung và danh sách terminal đủ rộng nên vẫn in chữ.

**Anh cần làm:** (a) Mở bảng, nhìn huy hiệu terminal trên một dự án và di chuột lên logo để thấy trạng thái (b) Báo nếu muốn chữ trạng thái quay lại (c) Đóng không kiểm tra. **Đề xuất:** (a) — một lần nhìn là đủ.

**Chuẩn xong:**
- Huy hiệu terminal chỉ có logo và tiêu đề, không còn chấm và chữ trạng thái
- Di chuột lên logo hiện tên agent kèm trạng thái
