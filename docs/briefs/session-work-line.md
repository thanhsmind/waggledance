---
title: Dải phiên đang chạy nói rõ mỗi phiên đang làm việc gì
audience: owner
size: S
ask: test
---

**Là gì:** Trước đây dải phiên đang chạy trên bảng chỉ cho biết phiên nào bận và thuộc nhánh việc nào. Nay mỗi dòng ghi tiêu đề việc phiên đó được giao, kèm trạng thái bằng chữ, và rê chuột lên dòng sẽ thấy tiêu chí nghiệm thu.

**Vì sao:** Khi nhiều phiên chạy cùng lúc, anh phải mở từng terminal mới biết phiên nào làm gì.

**Nguồn gốc:** Phần hiển thị cho tính năng ghi lại việc được giao mà bee thêm ngày 25/08/2026; quyết định sản phẩm chốt bên beehive.

**Chi phí và rủi ro:** Nhỏ, hai bước, đã vào nhánh chính. Chỉ đọc tiêu đề và tiêu chí, không đọc nội dung hội thoại; đường dẫn trong chữ được che. Phiên bee cũ chưa ghi việc thì dòng vẫn hiện như trước.

**Anh cần làm:** (a) Thử và chấp nhận (b) Báo lỗi. **Đề xuất:** (a) Thử và chấp nhận — chỉ cần nhìn trang chủ khi có vài phiên chạy.

**Chuẩn xong:**
- Mỗi dòng trong dải phiên đang chạy có tiêu đề việc
- Rê chuột lên dòng thì thấy tiêu chí nghiệm thu
- Phiên không có việc ghi lại vẫn hiện bình thường
