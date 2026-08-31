# Spec drop — sup-20260831-b2e1

**Provenance:** from waggledance@b3ab7b5
**Filed:** 2026-08-31
**Correlation id:** sup-20260831-b2e1 (same id as the registered PBI)
**Reference:** `docs/history/research/demonthorn-supervisor-xia.md` (this repo's own xia
brief the request is built on). Sibling drop: the `bee supervisor` half of the same xia
was filed in beehive under correlation id sup-20260831-7f3a — that half is not this
repo's concern.

## Request text (verbatim)

> ## Bối cảnh
>
> Một xia vừa chạy trên tài liệu `demonthorn-agent-orchestration-deep-dive.md` (mô hình
> Human / Supervisor / Lead / Peer). Brief đầy đủ:
> `docs/history/research/demonthorn-supervisor-xia.md` — đọc nó trước.
>
> Kết luận: toàn bộ ngữ nghĩa governance của supervisor **đã có sẵn trong bee**
> (`bee supervisor`: store append-only, frequency cap, mailbox, presence window,
> WakeReport, 7 health metric hai phía, silence-is-consent). Cái thiếu là **người đánh
> thức nó**. Phần bee đã được thả spec riêng sang beehive (corr-id sup-20260831-7f3a).
> Đây là phần của waggledance.
>
> ## Vì sao trigger thuộc waggledance chứ không thuộc bee
>
> Lựa chọn hiển nhiên là `bee herding control-loop --role supervisor` — loop đó đã có
> thật, Rust, bounded, có backoff. Nó thua ở hai điểm:
>
> - Nó **polling**, interval mặc định 60s. Đó đúng là §8.10 *polling/loop debt* và ngược
>   với §7.6 *sparse, event-driven supervision* của chính tài liệu nguồn.
> - Nó có `--main-root`, **neo vào một checkout**. Một loop mỗi repo = N supervisor mù về
>   nhau. Mà toàn bộ giá trị của role này (§9) là nhìn ngang qua nhiều workflow.
>
> waggledance là thứ duy nhất trong stack có **nhịp tim 24/7** và **registry biết mọi
> project**, cộng fleet read (`waggledance_ask_state`, `waggledance_runs`), dispatch door,
> và đường notify đã chứng minh.
>
> ## Hình dạng đề xuất
>
> 1. **Một background task mới** cạnh reaper và notify, theo đúng khuôn `reconcile_*` /
>    slot / cancel-flag / tick-counter mà ba task hiện có đang dùng chung
>    (`crates/waggledance/src/main.rs`).
> 2. **Event-driven, không phải timer.** Nó rình *transition* trên fleet — một run bị cap,
>    một run chuyển `blocked`, một run overrun, một escalation row mới xuất hiện — và
>    **chỉ khi đó** mới dispatch MỘT supervisor tick vào đúng repo xảy ra chuyện. Được phép
>    dùng sweep của reaper làm nguồn event, nhưng phải fire theo transition, không phải
>    mỗi interval một tick.
> 3. **Nó không phán xét gì cả.** Tick chạy qua dispatch door hiện có, agent bên kia đọc
>    record của repo đó và ghi bằng `bee supervisor record`. waggledance chỉ đánh thức.
> 4. **Switch opt-in, mặc định TẮT.** Nó gọi LLM — cùng hạng với `supervisor_enabled`
>    (spawn process) và `notify_enabled` (gọi ra ngoài). Chỉ reaper mới xứng đáng
>    default-on, và doc comment của `reaper_enabled` trong
>    `crates/waggledance-core/src/config.rs` giải thích rõ vì sao — đọc nó trước khi chọn
>    default. Vẫn phải bị `terminal.enabled` khống chế ở trên như mọi task nền khác.
> 5. **ĐỪNG đặt tên nó là supervisor.** `crates/waggledance/src/supervisor.rs` đã tồn tại
>    và là thứ hoàn toàn khác (watchdog ping/respawn herdr). Stack này đã có ba thứ tên
>    "supervisor"; đừng thêm cái thứ tư. Gọi đúng chức năng: trigger / observer-tick.
> 6. **Store ở lại bee.** Observation rơi vào `.bee/supervisor/` per-repo của từng project
>    — điều đó cưỡng chế đúng ràng buộc §9 (không dùng evidence của project A để accept
>    project B). waggledance không giữ bản sao nào.
>
> ## Cảnh báo về thứ tự — hãy đưa vào triage
>
> Xia khuyến nghị **giai đoạn zero-code trước**: hiện `.bee/supervisor/` chưa tồn tại ở
> bất kỳ repo nào, chưa một dòng record nào được ghi, nên **cả 7 counter đều đang
> `not-measurable`**. Xây trigger lúc này là tự động hoá một role chưa ai chứng minh được
> là đáng giữ, và rơi vào §8.11 *ceremony capture*. Chủ nhân đã xem cảnh báo này và vẫn
> yêu cầu làm đủ — nên nó được ghi ở đây như dữ liệu triage, không phải như một chặn.
> Repo này tự quyết có xếp nó sau một giai đoạn chạy tay hay không.

## Notes

- Filed as-is per the spec-drop contract. The ordering caveat in the request text above
  (zero-code / manual-run phase first, §8.11 ceremony-capture risk) is triage data, not a
  block — this repo's own triage decides whether to sequence behind a manual run.
- Naming constraint carried into triage: do not name the new task/module "supervisor" —
  `crates/waggledance/src/supervisor.rs` (herdr watchdog) already holds that name, and the
  `waggledance-supervisor` skill and `bee supervisor` CLI are two more unrelated uses.
  Candidate names from the request: trigger, observer-tick.
