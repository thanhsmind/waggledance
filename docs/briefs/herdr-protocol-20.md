---
title: Khởi động agent và mở shell mới qua herdr chạy lại được
audience: internal
size: M
ask: test
---

**Là gì:** herdr nâng cấp và đổi cách nhận lệnh tạo thẻ và khởi động agent, nên mọi lần khởi động agent hay mở shell mới từ WaggleDance đều bị từ chối. Bản chuyển đổi cho WaggleDance nói đúng ngôn ngữ herdr hiện tại: tạo thẻ trước, rồi khởi động agent vào ô vừa tạo.

**Vì sao:** Chỉ hai lệnh tạo mới hỏng nên không ai để ý; bảng vẫn ổn, chỉ nút Start, giao việc và mở shell là chết.

**Nguồn gốc:** Phát hiện ngày 25/08/2026 khi thử giao việc thật.

**Chi phí và rủi ro:** Vừa, bốn bước gộp một lần, đã vào nhánh chính và đã khởi động thật một agent. Không nới lỏng kiểm tra an toàn nào. herdr đổi tiếp thì lỗi có thể quay lại. Từ tháng 10 việc giao cho leader đi chủ yếu qua Paseo.

**Anh cần làm:** (a) Đồng ý đóng (b) Yêu cầu thử lại trước khi đóng. **Đề xuất:** (a) Đồng ý đóng — việc nội bộ, đã chạy thật, không còn gì phải làm.

**Chuẩn xong:**
- Bấm mở shell mới trong một dự án thì có ô terminal mới
- Giao việc tới agent herdr thì agent khởi động trong thẻ mới
- Việc này rời khỏi danh sách việc đang mở
