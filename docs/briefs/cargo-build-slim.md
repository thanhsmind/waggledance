---
title: Bản dựng thử nghiệm gọn hơn một nửa, chiếm ít ổ đĩa hơn
audience: internal
size: S
ask: test
---

**Là gì:** Trước đây mỗi lần agent dựng WaggleDance để chạy thử, thư mục dựng nặng khoảng 1,6 GB và tệp chạy 229 MB, vì mọi thư viện đi kèm mang đủ thông tin gỡ lỗi. Từ 23/9, thư viện ngoài bỏ phần đó: thư mục dựng còn khoảng 860 MB, tệp chạy 74 MB.

**Vì sao:** Máy phát triển đầy ổ nhanh và mỗi lần dựng lại tốn thời gian ghi đĩa; người chịu là các agent chạy song song.

**Nguồn gốc:** Việc dọn dẹp nhỏ agent tự làm ngày 23/9, đã gộp vào nhánh chính nhưng hồ sơ chưa được đóng.

**Chi phí và rủi ro:** Rất nhỏ, đã xong; bộ kiểm thử lúc gộp xanh toàn bộ. Rủi ro duy nhất: khi lỗi nằm trong thư viện ngoài, dấu vết lỗi thiếu số dòng ở đó. Anh không thấy gì thay đổi.

**Anh cần làm:** (a) Đồng ý đóng hồ sơ (b) Yêu cầu khôi phục cấu hình cũ. **Đề xuất:** (a) — đã chạy ổn gần năm tuần, không cần anh thử gì.

**Chuẩn xong:**
- Hồ sơ này được đóng, không còn hiện là việc đang mở
- Bản dựng thử nghiệm vẫn nhỏ hơn một nửa so với trước
