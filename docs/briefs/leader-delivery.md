---
title: Yêu cầu gửi leader đang bận được xếp hàng và tự báo lại khi xong
audience: owner
size: M
ask: test
---

**Là gì:** Trước đây gửi yêu cầu cho leader đang bận thì bị từ chối, và người gửi phải tự hỏi lại kết quả. Nay yêu cầu được nhận một lần, xếp hàng theo dự án, giao khi leader rảnh, đánh dấu xong khi lượt Paseo mang nó kết thúc, rồi tự báo lại người gửi. Gửi lại cùng yêu cầu không tạo việc trùng.

**Vì sao:** Việc bị từ chối hoặc lạc khi leader bận; lần thử cho thấy gửi chen vào làm leader bỏ dở việc đang làm.

**Nguồn gốc:** Bốn bài học đầu rút từ piggery, làm và gộp ngày 14/10, chưa được anh kiểm tra.

**Chi phí và rủi ro:** Vừa, đã xong, là nền của luồng wd giao thẳng leader hiện nay. Một việc treo quá một giờ bị đánh dấu mất.

**Anh cần làm:** (a) Kiểm tra rồi chấp nhận (b) Báo lỗi.
**Đề xuất:** (a) — luồng hằng ngày đã chạy trên nó.

**Chuẩn xong:**
- Gửi hai yêu cầu liên tiếp cho cùng dự án: cả hai chạy lần lượt
- Mỗi yêu cầu xong đều có thông báo trở về, không cần hỏi
- Khi WaggleDance tắt, yêu cầu báo đang tạm dừng thay vì im lặng
