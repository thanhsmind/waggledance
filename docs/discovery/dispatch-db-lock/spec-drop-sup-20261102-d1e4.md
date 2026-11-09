# Spec drop — sup-20261102-d1e4

**Provenance:** from waggledance@001760b
**Filed:** 2026-11-02
**Correlation id:** sup-20261102-d1e4 (same id as the registered PBI)
**Reporter:** a Claude session in `/home/thanhsmind/aicoworker/openclaw`, calling the
`waggledance_dispatch` MCP tool.

## Request text (verbatim)

> Lỗi gặp ngày 02/11/2026 khoảng 18:05–18:15 giờ VN, do phiên Claude ở
> /home/thanhsmind/aicoworker/openclaw gặp khi gọi MCP tool `waggledance_dispatch`:
>
> - Gọi `waggledance_dispatch {project: "goglbe-tools", leader: true, request_id:
>   "sup-20261102-b7d4", task: <~3k ký tự>}` ba lần cách nhau 1–3 phút, cả ba đều trả
>   lỗi: `run persistence failed: database error: database is locked`.
> - Cùng lúc đó một dispatch leader khác sang `agent-works` (request_id
>   sup-20261102-9c21) thành công ngay trước đó (run-b6c0c6d377a01ec6) và một run khác
>   đang chạy (run-a1577dc90e842ef1). Daemon đang bận giao việc/tạo leader Paseo cho
>   agent-works khi tôi gọi.
> - Đi vòng bằng `wd-bridge.py handoff --project goglbe-tools`
>   (workspace-wd/skills/waggledance-orchestrator/bin) thì queued ngay:
>   run-3c80addc3498a8d3, corr t-7a67d5.
> - Nghi: ghi SQLite đồng thời giữa daemon (leader queue/heartbeat) và handler MCP
>   dispatch không có busy_timeout hoặc WAL, nên ghi thứ hai thất bại tức thì thay vì
>   chờ.
>
> Mong muốn: dispatch không được rơi vì khoá DB tạm thời; ít nhất chờ (busy_timeout)
> hoặc thử lại nội bộ, và request_id giữ tính idempotent khi thử lại. Nếu đã có PBI
> tương tự thì gộp vào.

## Notes

- Overlaps with the already-proposed `p-7329a3c8` ("Dispatch fails with 'database is
  locked' right after the daemon starts"). Same error text; that item records
  `busy_timeout 5000 ms` already set and blames long write transactions during daemon
  startup. This drop is a second trigger: no restart, the daemon busy delivering a
  leader run to another project (agent-works), and three retries 1–3 minutes apart all
  failed — so the lock was held far longer than one busy_timeout window, or retries hit
  a fresh contention each time. The reporter asks that the two be merged; whether this
  drop merges into, supersedes, or stays beside `p-7329a3c8` is this repo's own triage
  call, not decided here.
- The reporter's suspicion (no busy_timeout, no WAL) is unverified data for triage, and
  `p-7329a3c8` already says busy_timeout is set.
- Carried requirement: a retry of `waggledance_dispatch` with the same `request_id`
  must stay idempotent — an internal retry, or a caller retry after a lock error, must
  never create a second run.
- Workaround that worked at the time: `wd-bridge.py handoff --project goglbe-tools`
  queued at once (run-3c80addc3498a8d3, corr t-7a67d5).
