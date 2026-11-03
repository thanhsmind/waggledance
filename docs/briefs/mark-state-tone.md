---
title: Logo agent đổi màu theo trạng thái: đang chạy, bị chặn hay rảnh
audience: owner
size: S
ask: test
---

**Là gì:** Trước đây logo của agent đang chạy và agent rảnh đều màu xám, nên chỉ chấm nhỏ bên cạnh cho biết ai đang làm. Nay logo mang luôn màu trạng thái: vàng cảnh báo khi đang chạy, đỏ khi bị chặn, xám khi rảnh.

**Vì sao:** Anh nhìn bảng để biết ai đang chạy, ai đang chờ; màu logo trả lời ngay.

**Nguồn gốc:** Một trong loạt chỉnh logo agent anh yêu cầu ngày 2026-08-25, đã vào nhánh chính cùng ngày. Ban đầu logo trên tiêu đề thẻ bảng giữ màu hãng; cuối ngày một quyết định sau cho logo đó cũng mang màu trạng thái. Nguồn chỉ có lịch sử commit và quyết định.

**Chi phí và rủi ro:** Nhỏ, đã làm xong. Đây là nền cho việc gỡ chấm trạng thái khỏi mọi terminal, nên nếu màu sai thì trạng thái khó đọc.

**Anh cần làm:** (a) Mở bảng và trang terminal khi có agent đang chạy và agent rảnh, xác nhận màu logo khác nhau (b) Báo lỗi nếu màu không đổi (c) Đóng không kiểm tra. **Đề xuất:** (a) — thay đổi dễ thấy nhất trong loạt này.

**Chuẩn xong:**
- Logo agent đang chạy màu vàng, bị chặn màu đỏ, rảnh màu xám
- Di chuột lên logo đọc được tên trạng thái
