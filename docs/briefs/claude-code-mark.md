---
title: Phiên chạy Claude Code hiện đúng logo Claude Code, không phải logo Claude
audience: owner
size: S
ask: test
---

**Là gì:** Khi một khung terminal chạy chương trình claude, WaggleDance từng có thể vẽ nhầm logo mặt trời của ứng dụng Claude. Nay chương trình claude luôn hiện logo Claude Code; logo mặt trời chỉ dành cho ứng dụng Claude.

**Vì sao:** Claude Code và ứng dụng Claude là hai thứ khác nhau. Vẽ nhầm làm anh tưởng một phiên đang chạy ứng dụng khác với thực tế.

**Nguồn gốc:** Phát sinh khi đưa logo thật lên bảng ngày 2026-08-25; quyết định chọn logo được ghi cùng ngày và đã vào nhánh chính trong cùng đợt thay đổi logo. Nguồn chỉ có lịch sử commit và quyết định.

**Chi phí và rủi ro:** Rất nhỏ, đã làm xong. Đây là phần tinh chỉnh của việc đưa logo thật lên bảng, nên kiểm tra chung với việc đó là đủ.

**Anh cần làm:** (a) Mở bảng hoặc trang terminal, nhìn một phiên Claude Code và xác nhận logo đúng (b) Báo lỗi nếu thấy logo mặt trời (c) Đóng mà không kiểm tra. **Đề xuất:** (a) — làm cùng lúc khi anh kiểm tra logo trên bảng.

**Chuẩn xong:**
- Phiên chạy claude hiện logo Claude Code ở thẻ, tab và danh sách terminal
- Logo mặt trời không xuất hiện cho phiên Claude Code
