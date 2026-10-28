---
title: Cổng kiểm tra mã xanh trên nhánh chính — đã làm, chờ anh đồng ý đóng
audience: internal
size: S
ask: test
---

**Là gì:** Trước đây ai chạy hai bước kiểm tra mã (rustfmt và clippy) trên nhánh chính sạch cũng thấy đỏ vì lỗi cũ. Ngày 31/8 một tính năng riêng đã dọn hết và hợp nhất; từ đó đỏ nghĩa là thay đổi mới gây ra.

**Vì sao:** Người làm, kể cả agent, mất thời gian truy lỗi không phải của mình. Mục này thuộc chính tính năng đã làm đó.

**Nguồn gốc:** Ghi nhận cuối tháng 8 khi nhiều tính năng liên tiếp vấp lỗi cũ; tính năng đã hợp nhất nhưng mục chưa đóng.

**Chi phí và rủi ro:** Không tốn thêm. Hôm nay kiểm tra định dạng vẫn sạch. Lưu ý: dự án là bản fork nên GitHub Actions chưa từng chạy các bước này; muốn giữ xanh lâu dài cần bật chạy tự động.

**Anh cần làm:** (a) Đồng ý đóng (b) Giữ mở tới khi bật chạy tự động. **Đề xuất:** (a), vì phần sửa đã vào nhánh chính, anh không cần làm gì.

**Chuẩn xong:**
- Mục này được đóng
- Hai bước kiểm tra mã chạy sạch trên nhánh chính
- Mục báo lỗi trùng nội dung được đóng cùng lúc
