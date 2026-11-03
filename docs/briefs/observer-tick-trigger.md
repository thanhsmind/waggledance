---
title: Tự đánh thức một lượt quan sát khi dự án có sự kiện đáng chú ý
audience: owner
size: L
ask: test
---

**Là gì:** Trước đây muốn agent theo dõi dự án thì phải chạy vòng lặp liên tục, tốn lượt gọi mô hình cả khi không có gì xảy ra. Từ 31/8, WaggleDance chỉ đánh thức một lượt quan sát khi có sự kiện: một việc kết thúc, agent kẹt chờ, việc chạy quá lâu, hoặc cảnh báo khẩn mới.

**Vì sao:** Anh biết sớm khi dự án gặp vấn đề mà không trả tiền cho agent canh suốt ngày.

**Nguồn gốc:** Từ nghiên cứu về giám sát thưa; anh duyệt ngày 31/8, kèm một ngoại lệ có ghi lại cho quy tắc không tự chọn việc giao.

**Chi phí và rủi ro:** Lớn, đã gộp, mặc định tắt. Chỉ chạy ở dự án đã đồng ý điều phối, có khoảng nghỉ giữa hai lượt. Rủi ro đã biết: một lượt ghi cảnh báo có thể đánh thức lượt kế, khoảng nghỉ giới hạn việc này.

**Anh cần làm:** (a) Bật kèm chạy thử một ngày, rồi nhờ agent đếm số lượt lẽ ra đã chạy (b) Để tắt và đóng (c) Bật thật. **Đề xuất:** (a) — biết số lượt thật trước khi chịu chi phí.

**Chuẩn xong:**
- Sau một ngày chạy thử, có con số lượt đánh thức
- Không agent nào bị gọi trong lúc chạy thử
