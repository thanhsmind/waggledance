---
title: Bảng không còn nhân đôi nút thao tác mỗi lần tự cập nhật
audience: owner
size: S
ask: test
---

**Là gì:** Hiện nay mỗi lần bảng tự cập nhật, cặp nút duyệt hoặc từ chối và nút chạy cạnh một thẻ lại bị thêm một bản sao, và cột đã trống vẫn giữ nút của thẻ đã rời đi. Bản sửa xoá các nút thừa đó, nên mỗi thẻ chỉ có một bộ nút.

**Vì sao:** Nút chồng nhau và nút mồ côi trong cột trống dễ khiến anh bấm nhầm.

**Nguồn gốc:** Lỗi lộ ra ngay sau khi bảng chuyển sang cập nhật tại chỗ, sửa ngày 25/08/2026.

**Chi phí và rủi ro:** Nhỏ, một bước, đã làm xong nhưng còn ở nhánh riêng; lỗi vẫn còn trên bản anh đang dùng. Nhánh đã cũ hai tháng nên lúc gộp có thể phải đối chiếu với các thay đổi bảng về sau.

**Anh cần làm:** (a) Thử rồi cho gộp (b) Cho gộp luôn (c) Bỏ. **Đề xuất:** (a) Thử rồi gộp — lỗi anh gặp hằng ngày, bản sửa nhỏ, chỉ cần vài phút quan sát.

**Chuẩn xong:**
- Để bảng mở khi vài phiên đang chạy, mỗi thẻ vẫn chỉ có một bộ nút
- Thẻ chuyển cột thì cột cũ không còn nút mồ côi
- Cột trống hiện lại dòng báo trống
