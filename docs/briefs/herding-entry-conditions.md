---
title: Agent khai báo dạng đầy đủ khởi động được và không treo ở màn hỏi tin cậy
audience: internal
size: M
ask: test
---

**Là gì:** Trước đây WaggleDance chỉ hiểu dạng khai báo agent đơn giản, nên agent khai báo kèm biến môi trường và yêu cầu tin cậy thư mục bị từ chối. Nay nó đọc cả hai dạng bee dùng, tự thêm thư mục sắp chạy vào danh sách tin cậy, và báo lại nếu bước đó thất bại.

**Vì sao:** Chỉ đọc dòng lệnh thì agent sẽ treo im lặng ở màn hỏi tin cậy, khó tìm nguyên nhân hơn lời từ chối cũ.

**Nguồn gốc:** Mở ngày 25/08/2026 từ một yêu cầu ghi sai bản chất lỗi.

**Chi phí và rủi ro:** Vừa, ba bước, đã vào nhánh chính và đã khởi động thật một agent trong beehive. Anh đã chấp nhận rủi ro chính: lần đầu WaggleDance ghi vào cài đặt của công cụ khác, nhưng chỉ thêm đúng một thư mục, không xoá.

**Anh cần làm:** (a) Đồng ý đóng (b) Yêu cầu kiểm tra lại trước khi đóng. **Đề xuất:** (a) Đồng ý đóng — việc nội bộ đã chạy ổn, quy trình đã khép.

**Chuẩn xong:**
- Agent khai báo dạng đầy đủ khởi động và bắt đầu làm
- Danh sách tin cậy chỉ thêm thư mục vừa chạy, không mất mục nào
- Việc này rời khỏi danh sách việc đang mở
