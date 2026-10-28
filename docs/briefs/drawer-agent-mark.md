---
title: Đóng việc thêm logo vào ngăn Agents, vì việc sau đã làm trọn
audience: owner
size: S
ask: drop
---

**Là gì:** Việc này cho mỗi dòng trong ngăn Agents mở đầu bằng logo agent, đứng trước chấm trạng thái. Đã vào nhánh chính ngày 2026-08-25.

**Vì sao:** Ngăn Agents là chỗ cuối cùng gọi agent bằng chữ trơn, nên cùng một agent trông khác nhau giữa bảng và ngăn.

**Nguồn gốc:** Một trong loạt chỉnh logo agent anh yêu cầu ngày 2026-08-25. Vì ngăn Agents được dựng trong trình duyệt, logo chỉ thật sự tới được ngăn nhờ việc kế tiếp cùng ngày; sau đó chấm trạng thái cũng bị gỡ, logo tự mang màu trạng thái. Không có hồ sơ thiết kế riêng, nguồn là lịch sử commit và quyết định đã ghi.

**Chi phí và rủi ro:** Không tốn thêm gì. Đóng việc này không gỡ gì; logo trong ngăn được kiểm tra qua việc đưa logo vào ngăn dựng trong trình duyệt.

**Anh cần làm:** (a) Đồng ý đóng vì việc sau đã làm trọn (b) Giữ lại để kiểm tra riêng. **Đề xuất:** (a) — kiểm tra một lần ở việc sau là đủ, hai thẻ cho cùng một thứ chỉ gây rối.

**Chuẩn xong:**
- Việc được đóng, ghi rõ việc nào đã làm trọn nó
- Mỗi dòng trong ngăn Agents vẫn mở đầu bằng logo agent
