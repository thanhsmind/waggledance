---
title: Đóng agent giám sát trong Paseo, đã thay bằng wd giao thẳng leader
audience: owner
size: L
ask: drop
---

**Là gì:** Ngày 17/10 một agent giám sát dài hạn trong Paseo được dựng để nhận việc từ wd, chọn dự án, mở leader, chuyển câu hỏi cho cố vấn hoặc cho anh qua Telegram, và đóng leader khi anh chấp nhận. Nó đã chạy thật một vòng từ wd tới leader và quay về. Ngày 22/10 anh quyết định bỏ tầng giám sát này; wd giờ giao thẳng cho leader.

**Vì sao:** Tầng trung gian chạy model rẻ hay bỏ sót tin, báo sai trạng thái và hỏi anh những câu cố vấn tự trả lời được.

**Nguồn gốc:** Anh yêu cầu ngày 17/10; bị thay bởi luồng wd giao thẳng leader ngày 22/10, luồng đó đã gỡ lệnh và hướng dẫn của agent giám sát.

**Chi phí và rủi ro:** Không tốn gì. Phần còn giá trị (chỉ đóng leader khi anh chấp nhận, các lệnh trả lời và làm lại) đã chuyển sang luồng mới.

**Anh cần làm:** (a) Bỏ, đóng hồ sơ (b) Giữ mở.
**Đề xuất:** (a) — đã được thay hoàn toàn.

**Chuẩn xong:**
- Hồ sơ không còn trên bảng việc
- Trong Paseo không còn agent giám sát nào chạy
- Việc giao qua wd vẫn tới leader và báo về Telegram
