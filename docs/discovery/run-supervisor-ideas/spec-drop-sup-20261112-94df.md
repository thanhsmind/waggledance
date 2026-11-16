# Spec drop — sup-20261112-94df

**Provenance:** from waggledance@601f92d
**Filed:** 2026-11-12
**Correlation id:** sup-20261112-94df (same id as the registered PBI)
**Origin:** a waggledance supervisor session, asking for an evaluation of forgent's Run
Supervisor against waggledance's own supervisor daemon.

## Request text (verbatim)

> Đánh giá: ý tưởng nào trong Run Supervisor của forgent dùng được để cải thiện supervisor daemon hiện tại của waggledance (heartbeat, reaper, wake, leader run lifecycle, hold, worktree sweep)?
>
> Nguồn tham khảo (đọc, không sửa): /mnt/data/Projects/refs/forgent/docs/platform/run-supervisor/spec.md (và docs/platform/seat-binding/spec.md mục 7.3 nếu cần).
>
> Tóm tắt Run Supervisor của forgent:
> - Một tiến trình Rust cho MỖI lần chạy một agent (attempt). Controller bàn giao rồi có thể chết; agent vẫn chạy, kết quả vẫn được settle, credential được ghi lại, home được dọn — không cần ai resume.
> - Bàn giao: controller lấy attempt lock + ghi attempt-intent.json ngay (để adopt dọn được nếu controller chết sớm), rồi ghi envelope.json bất biến có digest sha256 (lệnh, env không bí mật, deadlines, các bước hậu kỳ; secret để riêng 0600). Spawn supervisor bằng setsid, truyền lock qua fd, không pipe/IPC.
> - Gate: tiến trình gốc của agent (pid 1 trong PID namespace riêng qua bwrap trên Linux). Bắt tay HELLO/GO trước khi agent chạy. Gate giữ IO của agent nên supervisor chết thì agent không bị EPIPE; gate giữ hạn cứng (CLOCK_BOOTTIME tuyệt đối + boot id) làm chốt an toàn khi supervisor đã chết.
> - Canh 50 ms: hạn mềm (15'), idle (20' cho agent trong pane; chỉ chữ MỚI trên màn hình mới tính là hoạt động — spinner/đồng hồ/token counter không tính; màn hình không đọc được là "blind time", không tính idle cũng không tính hoạt động), giới hạn output 10 MiB, cancel. Phát hiện agent kẹt trong pane: chưa login, hết quota, hộp thoại trust, first-run setup → dừng sớm, ghi lý do cho người đọc.
> - Publication rule: mọi file trạng thái ghi temp + fsync + hard-link vào tên cuối; EEXIST nghĩa là đã có người ghi, đọc cái đó chứ không ghi đè (ghi đúng một lần, không race).
> - Receipt (kết quả vật lý, KHÔNG phải chấm bài): exited | timeout | cancelled | output-limit | gate-lost | abandoned-not-started | carrier-lost (pane mất) | abandoned-before-brief, kèm digest capture và reason (vd terminal:blocked:usage-limit). Kết quả trong outbox luôn thắng bất kể outcome.
> - Bước hậu kỳ theo thứ tự, mỗi bước có step receipt, chạy lại được: settle-attempt (chấm kết quả), finalize-home (ghi credential rồi dọn home), release-slot (đóng pane nếu chạy sạch, GIỮ pane nếu lỗi để người xem màn hình). closed.json khi mọi bước xong.
> - Adopt door: một lệnh nhận lại và settle mọi attempt mà supervisor đã chết, theo một ma trận khôi phục (lock tự do + domain rỗng mới được kết luận).
> - Chi phí: gần như không đáng kể (mili giây cho bàn giao/bắt tay; vài giây cho pane terminal và hậu kỳ).
>
> Câu hỏi cần đánh giá cho waggledance: phần nào (hạn cứng do tiến trình gốc giữ, idle theo chữ mới trên màn hình, receipt có tên lý do, publication một-lần, adopt door, giữ pane khi lỗi, phát hiện kẹt login/quota) lấp được lỗ hổng thật của supervisor daemon và vòng đời leader run hiện tại, phần nào thừa hoặc trùng với cái đã có (hold, reaper, lost/done của leader run). Không cần port nguyên — chỉ lấy ý tưởng khớp với vấn đề thật đã gặp.

## Acceptance (as stated by the requester)

- An evaluation, idea by idea, of these forgent Run Supervisor mechanisms against
  waggledance's supervisor daemon and leader run lifecycle: a hard deadline held by the
  agent's root process; idle measured only by new text on the screen (with unreadable
  screen time counted as neither idle nor activity); a physical-outcome receipt with a
  named reason; write-once publication of state files; an adopt door that settles runs
  whose supervisor died; keeping the pane open on failure; early detection of an agent
  stuck on login, quota, a trust dialog or first-run setup.
- For each idea, the evaluation says whether it closes a real gap that has actually been
  hit in waggledance, or duplicates or exceeds what already exists (hold, reaper, the
  leader run's lost/done states, heartbeat, wake, worktree sweep).
- The output names only the ideas that fit a real problem; a full port is explicitly not
  wanted.

## Notes

- The request is an evaluation, not a build. Whether it yields follow-up PBIs, and how
  many, is this repo's own triage call.
- The reference spec lives in another project. This session's cross-project guard
  refused to read `/mnt/data/Projects/refs/forgent/docs/platform/run-supervisor/`, so
  this drop carries only the requester's summary above. Whoever triages it needs that
  path added to `guards.cross_project_allow`, or must work from the summary alone.
- Domain neighbours already in the backlog, not merged or superseded here:
  `board-run-reaper` (board-dispatched runs never awaited, pane left open) and
  `p-1f6d0ebf` (await reports done from screen stability). Both touch the idle,
  receipt and pane-release ideas above.
