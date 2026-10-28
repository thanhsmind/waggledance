---
title: Dòng agent trên thanh bên quay về nằm gọn trên một dòng
audience: owner
size: S
ask: test
---

**Là gì:** Khi logo agent được thêm vào đầu mỗi dòng trong nhóm Agents trên thanh bên, bố cục bị lệch: logo và chấm trạng thái rơi lên một dòng riêng phía trên tên, chữ trạng thái tụt xuống theo. Bản sửa đưa logo, tên, địa chỉ và trạng thái về lại một dòng; dòng tính năng là dòng thứ hai duy nhất.

**Vì sao:** Dòng bị vỡ làm anh khó biết logo nào thuộc agent nào khi lướt thanh bên.

**Nguồn gốc:** Lỗi do chính loạt chỉnh logo agent ngày 2026-08-25 gây ra, sửa và đưa vào nhánh chính cùng ngày. Nguồn chỉ có lịch sử commit.

**Chi phí và rủi ro:** Rất nhỏ, chỉ chỉnh bố cục, đã làm xong. Ngay sau đó chấm trạng thái bị gỡ khỏi dòng này, logo mang luôn trạng thái, nên dòng còn gọn hơn lúc sửa.

**Anh cần làm:** (a) Mở WaggleDance, nhìn nhóm Agents trên thanh bên và xác nhận mỗi agent nằm trên một dòng (b) Báo lỗi nếu dòng vẫn vỡ (c) Đóng không kiểm tra. **Đề xuất:** (a) — chỉ cần liếc thanh bên.

**Chuẩn xong:**
- Logo, tên và trạng thái của mỗi agent nằm cùng một dòng
- Dòng tính năng, nếu có, là dòng thứ hai duy nhất
