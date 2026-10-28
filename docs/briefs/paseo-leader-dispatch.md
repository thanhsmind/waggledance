---
title: Gửi yêu cầu tới leader đúng dự án ngay từ Paseo
audience: owner
size: L
ask: test
---

**Là gì:** Trước đây WaggleDance chạy trong Paseo nhưng không giao được việc cho agent của Paseo. Nay mỗi dự án có một leader dài hạn trong workspace của nó; yêu cầu đến từ ba nơi: agent WaggleDance khi anh trò chuyện trong Paseo, ô gửi yêu cầu trong khung WaggleDance của Paseo, và agent bên ngoài như wd. Lượt chạy hiện trên bảng như mọi lượt khác.

**Vì sao:** Anh muốn nhận yêu cầu rồi giao cho đúng leader, đúng workspace, không phải tự mở từng nơi.

**Nguồn gốc:** Anh yêu cầu ngày 14/10; gộp cùng ngày. Hai đợt sau (xếp hàng khi leader bận, tự nối workspace) mở rộng nó.

**Chi phí và rủi ro:** Lớn, đã xong. WaggleDance không tự chọn dự án hay sửa nội dung việc, nên chọn sai dự án sẽ giao sai chỗ.

**Anh cần làm:** (a) Kiểm tra rồi chấp nhận (b) Báo lỗi.
**Đề xuất:** (a) — tính năng đã chạy trong luồng hằng ngày.

**Chuẩn xong:**
- Trong Paseo, mở khung WaggleDance của một dự án, gõ yêu cầu và gửi: leader của dự án nhận việc
- Lượt chạy đó hiện trên bảng việc trang chủ
- Gửi khi dự án chưa có leader: một leader mới được mở trong đúng workspace
