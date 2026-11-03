---
title: Thẻ trên bảng vẽ đúng logo thật của từng agent đang chạy
audience: owner
size: M
ask: test
---

**Là gì:** Trước đây thẻ trên bảng gọi agent bằng chữ hoặc một hình vẽ tay gần giống logo, dễ nhận nhầm. Nay mỗi thẻ vẽ logo thật của agent: Claude Code, Codex, opencode, Copilot, Cursor, Cline, Goose, Amp, Kilo Code; Gemini giữ ngôi sao vẽ tay vì không có bản gốc.

**Vì sao:** Anh nhìn bảng để biết ai đang làm gì; logo thật cho biết agent nào ngay từ cái nhìn đầu, không cần đọc chữ.

**Nguồn gốc:** Yêu cầu chỉnh giao diện của anh ngày 2026-08-25; đã vào nhánh chính cùng ngày. Nguồn chỉ có lịch sử commit và quyết định.

**Chi phí và rủi ro:** Đã làm xong, chỉ còn kiểm tra. Logo lấy từ bộ hình mã nguồn mở giấy phép Apache-2.0. Rủi ro duy nhất là daemon đang chạy bản cũ nên chưa hiện logo.

**Anh cần làm:** (a) Mở bảng trên WaggleDance, xem thẻ có agent đang chạy và xác nhận logo đúng (b) Báo lỗi nếu logo sai (c) Bỏ qua kiểm tra và đóng. **Đề xuất:** (a) — chỉ mất một phút.

**Chuẩn xong:**
- Thẻ có Claude Code hiện logo Claude Code, thẻ Codex hiện logo Codex
- Di chuột lên logo hiện tên chương trình
