---
artifact_contract: bee-research/v1
topic: piggery-lessons-xia
depth: standard
date: 2026-10-16
---

# piggery cải thiện được gì cho waggledance (phần thuộc waggledance)

Nguồn: spec drop `sup-20261016-19a3` từ `waggledance@e8b3751`. Quyết định khóa
của lần đánh giá này: `docs/history/piggery-lessons/CONTEXT.md`.

## Bottom Line

- **Recommendation (ladder rung): reuse.** Hình dạng piggery mà waggledance cần
  đã được mượn trong `leader-delivery` D1–D4. Phần còn lại đáng làm là ba sửa
  nhỏ trên chính bảng `runs` và đường delivery hiện có. Không có mục nào cần
  tích hợp piggery hay thêm một daemon thứ ba.
- **Vì sao đây là đường nhẹ nhất:** cả ba khoảng trống đều là chỗ waggledance
  chưa tuân luật của chính nó (D2 "never by reading idle state", D4 "tells the
  caller unasked") chứ không phải chỗ thiếu một hệ thống mail.
- **Vì sao rung kế tiếp thua:** "adapt-upstream" (mô hình team/gate, đọc
  `piggery ps --view`) giả định piggery đang chạy và có nhiều đội trong một
  project. Trên máy này piggery không được cài, và mỗi project chỉ có một leader.
- **Confidence: 80%.** Chắc ở phần Local (đọc code và fixture). Kém chắc ở hành
  vi của `paseo wait` sau khi một quyền được duyệt, vì chưa có capture (đó chính
  là PBI 1).
- **Suggested next step: bee-shaping**, từng PBI một khi được triage.

## Bảng kết luận

| # | Giả thuyết | Kết luận | Bằng chứng chính |
|---|---|---|---|
| 1 | Còn quy tắc piggery mà delivery chưa tuân và đang trả giá | **Đúng và đáng làm** (2 chỗ) | `delivery.rs:359` coi `idle` là done; `delivery.rs:450-503` giữ notice trong bộ nhớ |
| 2 | Dispatch qua gate của đội thay vì Paseo leader | **Không đáng** | `paseo.rs:904-909`: một leader cho mỗi project; D3 của leader-delivery đã có queue cho leader bận |
| 3 | Board đọc `piggery ps --view` / `tail --view` | **Sai tiền đề** | `command -v piggery` rỗng, không có `~/.piggery`; piggery chỉ chạy trên một máy |
| 4 | Notify kinds của piggery chính xác hơn cho cột "Waiting on you" | **Sai tiền đề, nhưng lộ ra một khoảng trống thật** | `server.rs:3496` board chỉ quét run `working`; run leader `blocked` không lên board |
| 5 | Skill supervisor mượn thêm kiểm tra/xếp hàng/idempotency | **Đã có tương đương** | `docs/waggledance-supervisor-skill-template.md:157-180` |
| 6 | `silent_for` + `notify: reports_to` so với observer-tick-trigger | **Đã có tương đương, không bổ sung** | observer-tick-trigger D3, D4(c); `paseo.turn_timeout_secs` |

## Repo Snapshot

- **Repo:** Rust workspace (`crates/waggledance-core`, `crates/waggledance`),
  daemon + MCP server + board web. Đường leader đi qua Paseo CLI 0.11.2.
- **Nguồn được đối chiếu:** piggery v0.10.2, bản local chỉ đọc tại
  `/mnt/data/Projects/refs/slp/piggery` (Go + SQLite + unix socket).
- **Ràng buộc:** chỉ Claude Code trong Paseo (CONTEXT D1). waggledance phục vụ
  board qua LAN và qua Cloudflare Access tunnel cho điện thoại
  (`docs/history/pi-paseo-plugin/CONTEXT.md` D7).

## Question & Assumptions

- **Điều được hỏi:** sáu giả thuyết ở trên, mỗi cái một kết luận có bằng chứng.
- **Success:** biết mục nào đáng làm, mục nào không, và mỗi mục đáng làm có một
  PBI `proposed`.
- **Giả định chưa xác nhận:** không có project nào của chủ nhân chạy piggery
  trên một máy khác. Bằng chứng duy nhất là máy này không có piggery.

## Findings

### Upstream (piggery v0.10.2)

- **Luật thiết kế** (`AGENTS.md:40-75`). Ba luật liên quan:
  - Luật 3: "A delivered batch is acknowledged only when the harness reports
    the end of that batch's turn … Never ack because a participant looks idle."
  - Luật 4: "State tables are the truth … nothing is rebuilt from events."
  - Luật 9: "Before relying on how a harness behaves … capture it from the real
    harness, commit it under `testdata/fixtures/<harness>-<version>/`, and test
    against the capture."
- **Gate to gate** (`docs/guide.md:164-178`). Một đội có một gate. Gate là
  session đã lập đội. Kênh duy nhất giữa hai đội là gate với gate. Khi gate vắng
  mặt, thư gửi tới đội được xếp hàng và không ai thay chỗ gate.
- **Notify** (`docs/guide.md:136-160`). Có bốn kind: `reply`, `settled`,
  `failed` và `gate_lost`. Chúng chạy hook trong `~/.piggery/hooks/notify.d/` và
  chỉ báo về turn trên mail team của gate. Một turn chat của người dùng không
  bao giờ sinh notify.
- **Timer** (`docs/reference.md:111`, `docs/guide.md:342`):
  `{on, silent_for, notify}`. Timer gửi một notice khi một member đang làm việc
  không kết thúc turn nào trong khoảng thời gian đó.
- **View JSON** (`docs/reference.md:21,23,41`). `ps --view` và `tail --view` in
  ra JSON có trường `version`, đúng thứ `top` hiển thị. Mọi lệnh đều có cờ
  `--no-start`, nghĩa là lệnh đọc mặc định sẽ khởi động daemon.
- **Một máy** (`AGENTS.md:6`): "One Go binary, one SQLite file, one unix socket."

### Local — từng giả thuyết

**H1. Bài học còn lại: đúng và đáng làm, ở hai chỗ.**

- *Luật 3: done được suy ra từ `idle` sau một permission.*
  `blocked_then_done` (`delivery.rs:342-370`) đợi permission được duyệt, gọi
  `paseo wait`, rồi chấp nhận `TurnEnd::Other { status: "idle" }` là done
  (`delivery.rs:359-361`). Đó chính là "ack because a participant looks idle".
  Nó cũng trái với chữ của `leader-delivery` D2: "never by reading idle state".
  Capture duy nhất của `wait` (`wait.out`) là từ một probe trước đó, không phải
  `wait` sau một permission đã được duyệt
  (`testdata/fixtures/paseo-0.11.2/README.md`, dòng `wait.out`). Vì vậy luật 9
  cũng chưa được tuân ở đúng nhánh này. Cái giá phải trả: nếu `wait` trả `idle`
  trước khi turn sau permission thật sự chạy tiếp, run bị đánh dấu `done` với
  câu trả lời rỗng, caller được báo xong, và task tiếp theo trong queue được gửi
  vào một leader vẫn đang dở việc.
- *Luật 4: trạng thái không nằm trong bảng.* Notice gửi trực tiếp cho Paseo agent
  đã gọi (D4) chỉ nằm trong `state.notices`, một `Vec` trong bộ nhớ
  (`delivery.rs:450-454`). Notice bị bỏ sau một giờ, chỉ để lại một dòng
  `tracing::warn` (`delivery.rs:462-464`), và mất khi daemon khởi động lại. Các
  trạng thái `quiet_until`, `headers` ("leader started in workspace …") và
  `not_before` (backoff) cũng chỉ nằm trong bộ nhớ. Thêm nữa, `end()` ghi
  transcript (`delivery.rs:437`) và status (`delivery.rs:441`) bằng hai câu
  UPDATE riêng (`repository.rs:263`, `repository.rs:389`), không trong cùng một
  transaction. Cái giá phải trả: D4 hứa "tells the caller unasked", nhưng một lần
  restart hoặc một caller bận hơn một giờ làm lời báo biến mất mà không ai biết.
  Đường notify bền (`orchestrate::enqueue_run_notice`, `orchestrate.rs:924`)
  chỉ tồn tại khi `notify_enabled` bật.
- *Đã tuân:* queue và claim nằm trong bảng `runs`
  (`claim_queued_run`, `repository.rs:214-217`, `UPDATE … WHERE status='queued'`).
  Done ở đường chính gắn với turn qua `send`/`run` blocking
  (`delivery.rs:288`, `delivery.rs:328`). Có 22 capture Paseo 0.11.2, và các test
  trong `paseo.rs:1944-2093` đọc chúng.
- *Đã khóa, không mở lại:* khi restart, mọi run đã gửi bị đánh dấu `lost` thay vì
  quan sát lại (`delivery.rs:182-203`). Đây là `leader-delivery` D6, nên bản
  đánh giá chỉ ghi lại, không đề xuất đổi.

**H2. Gate của đội thay vì Paseo leader: không đáng.**

- Mỗi project đúng một leader. `resolve_leader` lấy leader đầu tiên chưa archive
  theo nhãn (`paseo.rs:904-909`). Nếu không có, nó khởi động một leader kèm task
  (`delivery.rs:291-316`).
- Phần giá trị của mô hình gate là giữ thư cho người nhận bận hoặc vắng, và phần
  đó đã có: leader-delivery D3 (queue), D5 (không gửi vào turn đang chạy), D6
  (giữ queue khi leader có thể còn đang làm).
- Phần còn lại của mô hình gate, gồm nhiều đội trong một project, quyền ai gửi
  cho ai trong manifest, và relay qua gate, chỉ có ý nghĩa khi có nhiều đội
  ngang hàng. bee đã giữ phân công bên trong một project (cells, lanes, claims).
  Thêm "đội" sẽ tạo một hệ tọa độ thứ hai cạnh lanes.
- Khác biệt nên ghi lại: khi gate vắng, piggery giữ thư và không thay gate.
  waggledance thay một leader đã archive bằng leader mới. Cách của waggledance
  phù hợp hơn ở đây, vì leader là một agent máy mà Paseo luôn khởi động lại được.

**H3. Board đọc `piggery ps --view`: sai tiền đề.**

- Máy này không có piggery: `command -v piggery` không in gì, và
  `ls ~/.piggery` báo "No such file or directory". Không có consumer nào.
- Nếu đọc thì phải dùng `--no-start` (`reference.md:41`) để giữ luật "reading
  never starts a daemon" (`pi-paseo-plugin` D4). Như vậy tinh thần read-only vẫn
  giữ được về mặt kỹ thuật.
- Nhưng piggery chỉ chạy trên một máy với một unix socket (`AGENTS.md:6`), còn
  board waggledance được mở từ điện thoại qua tunnel. Board chỉ đọc được piggery
  của máy chạy daemon waggledance. PigMonitor đã làm đúng việc này bên trong
  Paseo, nên làm thêm ở đây là bản thứ hai của cùng một thứ.

**H4. Notify kinds cho cột "Waiting on you": sai tiền đề, nhưng lộ ra một khoảng
trống thật.**

- Cột chờ người lấy tín hiệu từ `BeeState.waiting_on_live`
  (`views.rs:1638-1716`), mark mà mỗi phiên bee tự ghi, và từ pane herdr ở trạng
  thái `blocked` (`views.rs:2583-2614`). Các kind của piggery chỉ mô tả mail
  team của piggery, và không có piggery nào chạy, nên chúng không thể là nguồn.
- Khoảng trống thật: board chỉ quét run `working`
  (`server.rs:3496`, `project_live_board_runs`). Một run leader Paseo ở `blocked`
  (leader đang chờ người duyệt quyền trong chat Paseo, `delivery.rs:344`) hay vừa
  `lost` ("the leader may still be finishing") không hiện ở đâu trên board. Người
  dùng chỉ biết qua notice trong bộ nhớ gửi cho agent đã gọi (H1). Đây đúng là
  tương đương của `failed`/`reply` bên piggery, và waggledance đã có dữ liệu đó
  trong bảng `runs`.

**H5. Skill waggledance-supervisor: đã có tương đương.**

- Template đã có các cơ chế của piggery. Skill truyền `request_id` bằng
  correlation id để gửi lại an toàn (`supervisor-skill-template.md:162`), một
  leader bận không bị từ chối mà run vào hàng `queued` (dòng 169-171), và skill
  chờ notice thay vì poll (dòng 175-180).
- Tương đương của bước kiểm tra trong piggery (`piggery why a b`) là
  `orchestration_enabled` của từng project cùng kiểm tra "leader mode
  unavailable" (dòng 208).
- Thứ duy nhất đáng mượn thêm là độ bền của notice (H1, PBI 2), không phải một
  cơ chế mới cho skill.

**H6. `silent_for` so với observer-tick-trigger: đã có tương đương.**

- Lớp D4(c) "run overrun" của observer-tick-trigger là một `working` run có tuổi
  vượt ngưỡng cố định. Đó là cùng tín hiệu với `silent_for`, đo ở mức run. Với
  leader, mỗi lần giao là một turn, nên tuổi run và thời gian từ turn end gần
  nhất trùng nhau.
- Đường leader còn có `turn_timeout_secs` (mặc định 1 giờ, `leader-delivery`
  D8). Hết giờ, run thành `lost` và caller được báo (`delivery.rs:327`).
- Lớp `notify: reports_to` (báo cho vai trò cấp trên) tương đương đường D4
  "báo caller", vốn đã có. Một timer thứ hai sẽ trái với D3 của
  observer-tick-trigger ("event-driven, never a timer").

### Inference

- Các khoảng trống ở H1 và H4 có cùng một gốc: một sự kiện mà người dùng cần
  biết (leader chờ quyền, leader mất) chỉ sống trong bộ nhớ hoặc trong chat
  Paseo, không nằm trong bảng `runs` mà board đọc. Sửa ở bảng thì cả notice lẫn
  board đều hưởng lợi. Đây là luật 4 của piggery áp vào chính waggledance.

## Cải thiện đáng làm (xếp theo giá trị / chi phí)

| Hạng | PBI | Giá trị | Chi phí | Vì sao |
|---|---|---|---|---|
| 1 | `p-929bf879` — Đánh dấu done sau permission chỉ bằng tín hiệu turn end đã capture | Cao: một done sai làm queue gửi task vào leader đang dở việc | Thấp: một capture và một nhánh trong `blocked_then_done` | Luật 3 + 9; chữ của leader-delivery D2 |
| 2 | `p-4cae7502` — Hiện run leader `blocked`/`lost` trong "Waiting on you" | Cao: hôm nay một leader chờ quyền là vô hình trên board và trên điện thoại | Thấp–vừa: một truy vấn trên `runs` cùng một pill/hàng trên board | H4; dữ liệu đã có |
| 3 | `p-f6975647` — Lưu notice cho caller trong store, không trong bộ nhớ | Vừa: D4 hiện có thể lặng lẽ không báo | Vừa: một bảng hoặc cột và đường flush đọc từ store | Luật 4; gộp luôn việc ghi transcript + status trong một transaction |

## Không nên làm, và vì sao

- **Tích hợp piggery làm hàng mail thứ hai.** Hai hàng song song (queue leader
  của waggledance và inbox của piggery) có thể giao cùng một việc hai lần hoặc
  theo thứ tự lệch nhau. `request_id` của waggledance không đi qua piggery,
  `client_msg_id` của piggery không đi qua waggledance. Không có lợi ích nào bù
  lại chuyện đó.
- **Mô hình team/gate cho `paseo-leader-dispatch`** (H2). Một project có một
  leader, và lanes của bee đã giữ phân công bên trong.
- **Board đọc `piggery ps --view`** (H3). Không có consumer, PigMonitor đã làm
  việc này, và piggery chỉ chạy trên một máy trong khi board hướng tới LAN và
  điện thoại.
- **Một daemon thứ ba.** waggledance và Paseo đã chia hai vai rõ ràng: board/MCP
  và agent runtime. Thêm piggery chỉ thêm một chỗ hỏng và một cổng socket thứ
  ba, không thêm năng lực nào mà ba PBI trên không đạt được với chi phí nhỏ hơn.
- **Timer `silent_for`** (H6). Trùng với overrun D4(c) và `turn_timeout_secs`,
  và trái với observer-tick-trigger D3.

## Risks, Unknowns, Follow-Ups

- **Chưa biết:** `paseo wait` trả gì khi turn sau permission còn đang chạy so
  với khi turn đã xong. PBI 1 phải capture điều đó trước khi sửa (luật 9).
- **Đụng quyết định khóa:** PBI 1 đưa code về đúng chữ của `leader-delivery` D2,
  không đổi D2. Không PBI nào đụng D6 (restart → lost).
- **Rủi ro được nêu trong spec:**
  - *Hai hệ mail song song:* tránh được, vì không tích hợp piggery.
  - *Một máy với LAN/điện thoại:* đây là lý do H3 bị loại.
  - *Daemon thứ ba:* bị loại, xem mục trên.
- **Nguồn bằng chứng:** mọi bằng chứng ở trên do phiên chính tự đọc trực tiếp.
  Digest piggery do agent phụ đọc về sau cùng khớp: timer theo
  `internal/.../watch.go:13-15`, notify theo `notify.go:144-149`, `--no-start`
  theo `cli/cli.go:97-106`. Digest đọc repo này bị dừng trước khi xong và không
  được dùng.

## Source Pack

- **Local:**
  - `crates/waggledance/src/delivery.rs`
  - `crates/waggledance/src/paseo.rs` (`resolve_leader`, fixture tests)
  - `crates/waggledance/src/server.rs` (`api_board`, `project_live_board_runs`)
  - `crates/waggledance/src/views.rs` (`proj_row_bee`, `pane_tone`)
  - `crates/waggledance/src/orchestrate.rs` (`terminal_from_stored`, `enqueue_run_notice`)
  - `crates/waggledance-core/src/repository.rs`
  - `crates/waggledance/testdata/fixtures/paseo-0.11.2/README.md`
  - `docs/waggledance-supervisor-skill-template.md`
  - `docs/history/leader-delivery/CONTEXT.md`
  - `docs/history/pi-paseo-plugin/CONTEXT.md`
  - `docs/history/observer-tick-trigger/CONTEXT.md`
- **Upstream** (piggery v0.10.2, local, chỉ đọc): `README.md`, `AGENTS.md`,
  `docs/guide.md`, `docs/reference.md`, `testdata/fixtures/`.
- **Docs:** không dùng tài liệu web. piggery được đọc trực tiếp từ bản local
  đúng phiên bản.
