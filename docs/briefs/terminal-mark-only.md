---
title: Terminal chỉ hiện logo agent thay cho tên chương trình in chữ
audience: owner
size: S
ask: test
---

**Là gì:** Trước đây huy hiệu terminal, tab khung và danh sách terminal của một tính năng in cả logo lẫn tên chương trình, nói một điều hai lần. Nay chỉ còn logo; tên chương trình hiện khi di chuột lên và vẫn được đọc cho trình đọc màn hình.

**Vì sao:** Dòng terminal ngắn hơn, chỗ trống dành cho tiêu đề việc đang làm, thứ anh thật sự cần đọc.

**Nguồn gốc:** Một trong loạt chỉnh logo agent anh yêu cầu ngày 2026-08-25, thay cho việc đặt logo cạnh tên chữ làm trước đó cùng ngày. Đã vào nhánh chính cùng ngày. Nguồn chỉ có lịch sử commit và quyết định.

**Chi phí và rủi ro:** Nhỏ, đã làm xong. Rủi ro là logo lạ khó nhận ra khi thiếu chữ; di chuột vẫn cho ra tên.

**Anh cần làm:** (a) Mở trang terminal của một dự án, xem tab và danh sách terminal chỉ còn logo, di chuột để thấy tên (b) Báo lỗi nếu thấy thiếu tên khi di chuột (c) Đóng không kiểm tra. **Đề xuất:** (a) — kiểm tra nhanh, cùng lúc với các chỉnh logo khác.

**Chuẩn xong:**
- Tab khung và danh sách terminal không còn in tên chương trình cạnh logo
- Di chuột lên logo hiện tên chương trình
