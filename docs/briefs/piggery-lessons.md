---
title: Bảng việc báo khi leader đang chờ quyền hoặc mất liên lạc
audience: owner
size: M
ask: test
---

**Là gì:** Sau khi đối chiếu với piggery, một công cụ cùng loại, ba lỗi trên đường giao việc cho leader được sửa ngày 16/10. Nay bảng việc trên trang chủ hiện một nhãn ở dòng dự án khi leader đang chờ cấp quyền hoặc mất liên lạc trong giờ qua; thông báo cho người giao việc được lưu bền nên không mất khi máy khởi động lại; và bước chờ sau khi cấp quyền được kiểm chứng bằng bản ghi thật của Paseo.

**Vì sao:** Trước đó leader kẹt chờ quyền hoàn toàn vô hình trên bảng; anh chỉ biết khi tự mở Paseo.

**Nguồn gốc:** Anh gửi yêu cầu đánh giá piggery ngày 16/10, rồi bảo "Vậy fix 3 lỗi đó" cùng ngày; đã gộp nhưng chưa kiểm tra.

**Chi phí và rủi ro:** Vừa, đã xong. Rủi ro thấp: nhãn chỉ đọc từ danh sách lượt chạy sẵn có.

**Anh cần làm:** (a) Kiểm tra rồi chấp nhận (b) Báo lỗi.
**Đề xuất:** (a) — sửa đúng chỗ anh từng không thấy.

**Chuẩn xong:**
- Khi một leader chờ cấp quyền, dòng dự án trên bảng có nhãn chờ quyền
- Leader mất liên lạc trong giờ qua hiện nhãn mất
- Khởi động lại WaggleDance khi còn thông báo chưa gửi: thông báo vẫn đến
