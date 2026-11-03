---
title: Ngăn Agents hiện logo agent và đổi màu theo trạng thái
audience: owner
size: S
ask: test
---

**Là gì:** Ngăn Agents được dựng trong trình duyệt nên trước đây không nhận được logo agent như bảng và terminal. Nay mỗi dòng trong ngăn có logo thật của agent.

**Vì sao:** Cùng một agent giờ trông giống nhau ở bảng, terminal và ngăn Agents.

**Nguồn gốc:** Một trong loạt chỉnh logo agent anh yêu cầu ngày 2026-08-25, đã vào nhánh chính cùng ngày. Ngày 2026-08-31 có một bản sửa: logo trong ngăn bị tô đen nên agent đang chạy và agent rảnh trông giống hệt nhau; bản sửa cho logo nhận màu trạng thái. Nguồn chỉ có lịch sử commit và quyết định.

**Chi phí và rủi ro:** Nhỏ, đã làm xong kể cả bản sửa màu. Rủi ro là daemon đang chạy bản trước ngày 2026-08-31 nên logo vẫn đen.

**Anh cần làm:** (a) Mở ngăn Agents khi có một agent đang chạy và một agent rảnh, xác nhận logo hiện và khác màu nhau (b) Báo lỗi nếu logo đen (c) Đóng không kiểm tra. **Đề xuất:** (a) — một lần mở ngăn là đủ.

**Chuẩn xong:**
- Mỗi dòng trong ngăn Agents có logo agent đúng
- Logo agent đang chạy mang màu cảnh báo, agent rảnh màu xám
