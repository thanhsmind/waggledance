---
title: Cho một dự án chạy nhiều leader song song khi việc được đánh dấu độc lập
audience: owner
size: L
ask: park
---

**Là gì:** Hiện mỗi dự án chỉ có một leader trong Paseo, hai việc cùng dự án phải xếp hàng. Sau thay đổi, việc đánh dấu độc lập được mở thêm leader chạy song song.

**Vì sao:** Khi anh giao nhiều việc nhỏ không liên quan cho một dự án, việc sau phải đợi việc trước.

**Nguồn gốc:** Ghi lại ngày 2026-10-17 khi thiết kế supervisor trong Paseo, thiết kế chốt một leader mỗi dự án. Supervisor bị gỡ ngày 2026-10-22, quy tắc đó vẫn giữ.

**Chi phí và rủi ro:** Lớn, nhiều đợt. Cần trước một quy tắc chống va chạm: hai leader cùng sửa một nhánh hay cùng ghi trạng thái bee sẽ hỏng việc của nhau. Chưa ai thiết kế quy tắc đó.

**Anh cần làm:** (a) Duyệt làm (b) Hoãn (c) Bỏ. **Đề xuất:** (b) Hoãn — hàng chờ một leader vẫn đủ dùng, phần khó nhất chưa có thiết kế.

**Chuẩn xong:**
- Có quy tắc rõ khi nào hai leader được chạy cùng lúc trong một dự án
- Hai việc độc lập cùng dự án chạy song song không đè nhau
- Việc không đánh dấu độc lập vẫn xếp hàng như hôm nay
