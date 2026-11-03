---
title: Bảng tự cập nhật tại chỗ, không chớp trang, thẻ trượt sang cột mới
audience: owner
size: M
ask: test
---

**Là gì:** Trước đây mỗi khi có thay đổi, bảng tải lại cả trang, màn hình chớp và anh mất chỗ đang đọc. Nay bảng ở trang chủ và bảng từng dự án cập nhật tại chỗ: thẻ đổi cột thì trượt sang, thẻ mới hiện dần, thẻ rời đi mờ dần.

**Vì sao:** Anh để bảng mở liên tục; cái cần thấy là việc nào vừa chuyển, không phải cả trang nhấp nháy.

**Nguồn gốc:** Làm ngày 25/08/2026, hai quyết định về chuyển động do anh chốt.

**Chi phí và rủi ro:** Vừa, hai bước, đã vào nhánh chính. Trang tài liệu, tìm kiếm và mã nguồn vẫn tải lại như cũ. Ngay sau đó lộ ra lỗi nút thao tác bị nhân đôi; bản sửa là việc riêng đang chờ anh thử.

**Anh cần làm:** (a) Thử và chấp nhận (b) Báo lỗi. **Đề xuất:** (a) Thử và chấp nhận — đã chạy hai tháng; nút trùng là lỗi của việc sửa riêng kia.

**Chuẩn xong:**
- Để trang chủ mở khi vài phiên chạy: trang không chớp, chỗ cuộn giữ nguyên
- Thẻ đổi cột thì trượt sang thay vì nhảy
- Bật giảm chuyển động của máy thì bảng vẫn cập nhật nhưng không trượt
