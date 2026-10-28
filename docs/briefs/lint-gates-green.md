---
title: Đợt làm xanh hai bước kiểm tra mã tháng 8 đã xong — nay đỏ lại một dòng
audience: internal
size: S
ask: test
---

**Là gì:** Cuối tháng 8, hai bước kiểm tra tự động của dự án, kiểm tra định dạng mã và kiểm tra chất lượng mã, đều báo đỏ trên nhánh chính do mã cũ. Ngày 31/8 agent đã sửa và gộp: định dạng lại, xóa vài dòng thừa. Hôm nay bước định dạng vẫn xanh, nhưng bước chất lượng đỏ lại vì một dòng thừa mới do một thay đổi về sau.

**Vì sao:** Khi hai bước này đỏ sẵn, không ai phân biệt được lỗi mới với lỗi cũ, và mọi agent sau đều làm việc trên nền đỏ.

**Nguồn gốc:** Agent phát hiện ngày 31/8 khi chạy kiểm tra trước lúc gộp; sửa và gộp cùng ngày, hồ sơ chưa đóng.

**Chi phí và rủi ro:** Hồ sơ tháng 8 không còn việc gì. Lỗi đỏ hôm nay là việc mới, đã có một mục trong danh sách chờ xin duyệt sửa một dòng.

**Anh cần làm:** (a) Đồng ý đóng hồ sơ tháng 8 (b) Giữ mở đến khi dòng đỏ mới được sửa. **Đề xuất:** (a), vì việc tháng 8 đã xong và lỗi mới đã có thẻ riêng.

**Chuẩn xong:**
- Hồ sơ tháng 8 được đóng
- Lỗi đỏ mới được theo dõi ở đúng một mục trong danh sách chờ
