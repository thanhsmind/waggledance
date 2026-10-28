---
title: Dòng trong ngăn Agents xếp trạng thái nhỏ phía trên tên agent
audience: owner
size: S
ask: test
---

**Là gì:** Trước đây trạng thái và tên agent nằm cạnh nhau trên một dòng, giành chỗ của nhau. Nay chúng xếp chồng: logo bên trái, trạng thái chữ nhỏ ở trên, tên agent to nhất ở dưới, rồi dòng tính năng như cũ.

**Vì sao:** Tên là phần duy nhất khác nhau giữa các dòng, nên nó cần nổi nhất để anh lướt ngăn Agents tìm đúng agent.

**Nguồn gốc:** Yêu cầu chỉnh giao diện của anh ngày 2026-08-25, đã vào nhánh chính cùng ngày. Lúc đó chấm màu trạng thái vẫn giữ; chiều cùng ngày chấm bị gỡ khỏi mọi terminal và logo tự mang màu trạng thái, phần xếp chồng giữ nguyên. Nguồn chỉ có lịch sử commit.

**Chi phí và rủi ro:** Nhỏ, đã làm xong. Không phụ thuộc việc nào đang mở.

**Anh cần làm:** (a) Mở ngăn Agents, xem mỗi dòng có trạng thái nhỏ ở trên và tên agent ở dưới (b) Báo nếu muốn đổi lại thành một dòng (c) Đóng không kiểm tra. **Đề xuất:** (a) — thay đổi đã chạy hai tháng, chỉ cần anh xác nhận bố cục ổn.

**Chuẩn xong:**
- Mỗi dòng trong ngăn Agents có trạng thái nằm trên tên agent
- Tên agent là chữ lớn nhất trong dòng
