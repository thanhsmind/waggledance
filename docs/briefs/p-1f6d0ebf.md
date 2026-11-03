---
title: Chỉ báo việc xong khi agent tự nói xong, không đoán từ màn hình đứng yên
audience: internal
size: M
ask: park
---

**Là gì:** Với việc chạy trong khung terminal herdr, WaggleDance hiện báo "xong" khi màn hình đứng yên khoảng một giây rưỡi, kể cả khi agent đang dừng chờ công cụ. Sau thay đổi, chỉ báo xong khi agent phát tín hiệu hoàn thành rõ ràng.

**Vì sao:** Báo xong sớm khiến bên gọi đọc kết quả dở dang. Trên máy này herdr không nhận diện được agent nào, nên phép đoán áp dụng cho mọi khung.

**Nguồn gốc:** Lộ ra ngày 2026-08-30 khi sửa phần gửi việc và thu hồi khung; anh chốt khi đó "xong" phải là lời tuyên bố rõ ràng, nhưng gỡ phép đoán vượt phạm vi.

**Chi phí và rủi ro:** Vừa, vài phần việc. Bỏ phép đoán thì agent không phát tín hiệu xong sẽ treo tới hết giờ chờ, cần cách xử lý riêng. Việc giao cho leader Paseo không đi qua đường này.

**Anh cần làm:** (a) Duyệt làm (b) Hoãn (c) Bỏ. **Đề xuất:** (b) Hoãn — việc của anh nay chủ yếu đi qua leader Paseo.

**Chuẩn xong:**
- Việc chỉ được báo xong khi có tín hiệu hoàn thành rõ ràng
- Agent không phát tín hiệu xong có cách xử lý được kiểm thử
- Agent dừng chờ công cụ không còn bị báo xong
