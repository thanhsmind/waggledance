---
title: Bộ kiểm tra định dạng và lint của repo xanh lại trên nhánh chính
audience: internal
size: S
ask: build
---

**Là gì:** Repo có hai bước kiểm tra chất lượng mã: định dạng và lint nghiêm ngặt. Hôm nay định dạng xanh, nhưng lint đỏ vì một lỗi nhỏ trong phần dựng giao diện. Sau khi sửa, cả hai xanh, nên bước nào đỏ thì chắc chắn do thay đổi mới.

**Vì sao:** Khi kiểm tra đã đỏ sẵn, người đóng góp không phân biệt được lỗi của mình với lỗi cũ. Repo là bản fork nên GitHub chưa từng tự chạy hai bước này.

**Nguồn gốc:** Thuộc đợt làm xanh bộ kiểm tra, hợp nhất ngày 2026-08-31 khi cả hai đã xanh. Kiểm tra lại ngày 2026-10-28 thấy lint đỏ với một lỗi mới.

**Chi phí và rủi ro:** Rất nhỏ: sửa một dòng, không đổi hành vi. Không có CI chạy lint nên nó có thể đỏ lại.

**Anh cần làm:** (a) Duyệt làm (b) Hoãn (c) Bỏ. **Đề xuất:** (a) Duyệt làm — một dòng là đóng được mục này và đợt làm xanh đi kèm.

**Chuẩn xong:**
- Kiểm tra định dạng xanh trên nhánh chính
- Kiểm tra lint nghiêm ngặt xanh trên nhánh chính
