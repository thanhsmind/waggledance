---
artifact_contract: bee-research/v1
topic: demonthorn-supervisor-xia
depth: standard
date: 2026-08-31
---

## Bottom Line

- **Recommendation (ladder rung): reuse.** Trách nhiệm "supervisor như một agent
  giám sát" mà Demonthorn mô tả **đã tồn tại gần như trọn vẹn** trong stack này —
  không phải ở waggledance, mà ở `bee supervisor` (10 verb, store append-only,
  mailbox có frequency cap, presence window, WakeReport, 7 health metric suy ra
  từ record). Cái thiếu không phải cơ chế, mà là **người ngồi vào ghế**: không có
  skill nào lái nó, và store chưa từng được ghi một dòng.
- **Vì sao đây là đường nhẹ nhất:** rung 1 thắng vì phần khó nhất của mô hình
  Demonthorn — governance plane tách khỏi execution plane, có authority boundary
  cưỡng chế được — đã được cài đặt và cưỡng chế ở tầng CLI. Viết mới sẽ là bản
  thứ hai của cùng một thứ.
- **Vì sao rung kế tiếp thua:** "adapt-upstream" (bê profile TOML + notebook shape
  của Demonthorn vào) thua vì nó giả định một control plane kiểu Paseo
  (agent/session/workspace/parentage là first-class object). bee không có
  parentage; nó có lane/claim/worktree. Bê nguyên sẽ tạo hai hệ toạ độ song song.
- **Confidence: 80%.** Chắc về phần Local (đọc trực tiếp CLI help + store +
  config). Kém chắc về ý định của tác giả doc ở chỗ Supervisor "tạo Lead mới" —
  phần đó không có đối ứng local nào và doc tự gắn nhãn [SYNTHESIS].
- **Suggested next step: none** — đây là xia, kết thúc bằng thảo luận. Nếu muốn
  thực sự dựng ghế supervisor thì đó là một feature riêng, vào bee-shaping.

## Repo Snapshot

- **Repo type / languages:** Rust workspace (`crates/waggledance-core`,
  `crates/waggledance`), edition 2021, version 0.5.2; tooling agent là bee
  v2.29.0 (CLI Node/JS tại `.bee/bin/bee`).
- **Nguồn được distill:** `/home/thanhsmind/Projects/refs/slp/paseo-pi-team/docs/demonthorn-agent-orchestration-deep-dive.md`
  — 1301 dòng, tiếng Việt, tự gắn nhãn `[DIRECT]` / `[SYNTHESIS]`. Không phải
  repo code; là tài liệu thiết kế mô tả mô hình Demonthorn chạy trên Paseo.
- **Ràng buộc định hình câu trả lời:** paseo đã bị gỡ khỏi waggledance hoàn toàn
  (commit `57f3193`, decision `paseo-removal D3/D4`). Mọi khuyến nghị bám vào
  Paseo-as-control-plane trong doc **không** áp dụng trực tiếp được nữa.

## Question & Assumptions

- **Điều được hỏi:** supervisor trong doc này có nhiều trách nhiệm hơn và là một
  *agent* để theo dõi và hành động — khác với hai thứ tên "supervisor" ở
  waggledance. Nó thực sự là gì và làm gì?
- **Success trông như thế nào:** hiểu đúng ranh giới trách nhiệm của role đó, và
  biết stack hiện tại đã có/chưa có gì tương ứng.
- **Giả định chưa xác nhận:** rằng người hỏi đang so sánh để hiểu, chưa yêu cầu
  mang về. Brief này dừng ở đó.

## Findings

### Local

**Ba thứ khác nhau cùng tên "supervisor" trong stack này** — và cái Demonthorn
nói **không phải** hai cái đầu:

| # | Tên | Là gì | Bản chất |
|---|---|---|---|
| 1 | herdr supervisor | `crates/waggledance/src/supervisor.rs`, 424 dòng Rust | watchdog tất định: ping herdr mỗi 5s, chết thì spawn lại, backoff 3s→60s. Không LLM. Đang **tắt** (`supervisor_enabled = false`) |
| 2 | `waggledance-supervisor` skill | `~/.claude/skills/waggledance-supervisor/SKILL.md` | file markdown, one-shot relay: thả spec vào repo đích thành PBI `proposed` rồi dừng. Không control loop |
| 3 | **`bee supervisor`** | 10 verb CLI + `.bee/supervisor/` store | **đây mới là đối ứng của Demonthorn** |

**`bee supervisor` — đối ứng thật, và nó đã đi xa hơn doc ở vài chỗ:**

- `record --kind observation|silence` → `observations.jsonl`. "Nhìn rồi chọn im
  lặng" là **outcome hợp lệ được ghi**, không phải tick rỗng — đúng tinh thần
  §7.6 "sparse supervision", nhưng bee biến nó thành record kiểm chứng được.
- `record --kind intervention|escalation|urgent` → mailbox `interventions.jsonl`,
  cần `--target-session --point-key --question`. **Frequency cap:** intervention
  thứ hai cho cùng `(session, point-key)` bị **từ chối**, remedy duy nhất là
  nâng lên `escalation`. Doc §8.17 *khuyên* đừng micromanage; bee *chặn* nó.
- `--question` bị giới hạn **tối đa hai câu**. Validation chạy **trước** khi
  append — sai kind/signal/slug/độ dài là refuse và không ghi gì.
- `pending` / `mark-delivered`: session đích đọc mailbox ở turn boundary. Append-only,
  mark lại là no-op `changed:false`.
- `away` / `back` / `presence`: presence window. Trong lúc away, mọi row **non-urgent**
  bị stamp `queued`; `urgent` không bao giờ bị queue và bắn desktop notification.
  Presence có **đúng hai tác dụng** (decision `9f5cd250`): định nghĩa report window,
  và làm câu hỏi non-urgent xếp hàng. Nó **không** đụng gate, bypass level,
  permission hay waiting-on — "permission control never hides in a presence flag".
- `report`: WakeReport markdown, **tối đa 10 dòng, đúng 4 mục** — What happened /
  What was decided / What needs you / Next action. Item sắp theo impact-if-wrong
  giảm dần (decision `66c4c251`). Render **một lần** lúc `back`, đọc lại luôn ra
  cùng bytes.
- `metrics`: **7 counter, mỗi cái có band hai phía**, suy ra tất định từ cell
  file / decision log / mailbox / observation store — *"nothing here is
  model-supplied and nothing is self-reported"* (decision `a8f4b8ab`).
  `not-measurable` là verdict hạng nhất, không bao giờ render thành `in-band`.
  Đáng chú ý nhất: **below-band được báo to đúng bằng above-band, vì "a supervisor
  that never speaks is as broken as one that never stops"** — đây là thứ doc
  Demonthorn không có.
- `consent-sweep`: silence-is-consent, **opt-in, mặc định tắt, fail closed**.
  Scope hẹp và mỗi loại trừ bị từ chối đích danh: gate không bao giờ đi đường này,
  urgent/escalation cũng không, one-way door ở confidence thấp cũng không.

**Ranh giới authority được cưỡng chế ở tầng tool**, không phải ở tầng lời khuyên:
`bee supervisor record` tự mô tả *"This verb observes and asks — it dispatches
nothing, approves nothing, and writes no other bee record."* Đó chính xác là
§8.17 (Supervisor overreach) được đóng cứng thành API.

**Role đã được cấu hình sẵn:** `.bee/config.json:57` có
`models.claude.roles.supervisor = { model: "haiku", description: "cold observer
tick — structured observation on a cheap model; decides nothing, escalation rides
the advisor role" }`. Khớp gần như nguyên văn §4.2: *"Supervisor có thể dùng model
rẻ hơn nếu chỉ làm monitoring có cấu trúc."*

**Và đây là khoảng trống thật:**

- `.bee/supervisor/` **không tồn tại** — store chưa từng được ghi. `bee supervisor
  presence` trả `state: present, window: null, queued: 0`.
- **Không có skill nào lái nó.** 13 skill trong `.claude/skills/`, `rg "supervisor"`
  trên toàn bộ thư mục đó ra **rỗng**. Không có `bee-supervising`.
- Không tìm thấy hook nào đọc `bee supervisor pending` ở turn boundary — nghĩa là
  mailbox có thể ghi được nhưng **chưa có ai đọc**.

### Upstream

Nguồn là tài liệu thiết kế, không phải repo code — nên không có "upstream proof"
theo nghĩa code chạy được. Cấu trúc role của nó:

```
                         Human
              ┌────────────┴────────────┐
        Supervisor                  Project Lead
   governance / observation      project authority
              └──── observes ───────────┤
                                    Peer(s)
```

**Không phải hierarchy cứng.** Doc nói rõ: Lead là "god" trong workspace của nó;
Supervisor giám sát và *có thể điều chỉnh* Lead; hai role có **hai loại authority
khác nhau**; Human giữ owner authority cuối.

**Supervisor SỞ HỮU (§4.2 Authority hợp lý):**
quan sát nhiều project/workspace · hỏi Lead vì sao chọn strategy · báo Human về
bias/risk · relay owner decision tới Lead · đề xuất patch profile/protocol · ghi
notebook pattern + causal context · (chỉ khi Human cho phép) tạo Lead mới và
handoff nếu Lead cũ không recover được.

**Supervisor KHÔNG sở hữu mặc định (§4.2):**
implementation scope · architecture decision · project acceptance · quyền sửa code
"cho nhanh" · quyền biến hypothesis thành correction order khi chưa reconcile
evidence.

**Output shape bắt buộc (§4.2):** `Observation / Evidence / Suspected mechanism /
Impact / Question for Lead / Recommendation / Escalation needed?`

**Notebook phải có causal context, không chỉ verdict** — *"chỉ lưu verdict khiến
hệ thống học câu khẩu hiệu và dễ phản ứng quá mức"*. Ví dụ của doc lưu cả
Observation → Cause evidence → Anti-pattern → Recovery → **Protocol candidate**.

**Profile mẫu (§4.2, [SYNTHESIS]):** `sandbox_mode = "read-only"`,
`approval_policy = "never"`, `model_reasoning_effort = "medium"`,
`[agents] enabled = false` — supervisor **không được spawn agent**.

**Điều Supervisor tìm (§4.2 Góc nhìn):** Lead–Peer conversation · session history ·
git/workspace history · repeated tool failures · **loss of momentum** · recurring
anti-pattern · **decision bị bỏ quên qua compaction/handoff**.

### Docs

Không có official docs nào version-matched để đối chiếu: nguồn là tài liệu nội bộ
tái dựng, tự nhận phần lớn profile/template là `[SYNTHESIS]` chứ không phải nội
dung nguyên văn để copy. Không có web research nào được chạy cho brief này — đây
là capability gap được ghi nhận, không phải bước bị bỏ im lặng.

### Dependency matrix

| Trách nhiệm (Demonthorn §4.2/§7.6) | Đối ứng local | Verdict | Evidence |
|---|---|---|---|
| Quan sát nhiều workspace/session | `waggledance_ask_state` / `waggledance_runs` (fleet read); bee lane/claim/session list | `EXISTS` (rời rạc, chưa hợp thành một view) | `Local` |
| Phát hiện anti-pattern | `bee supervisor metrics` — 7 counter suy ra tất định | `EXISTS` (mạnh hơn doc) | `Local` |
| Hỏi Lead câu hỏi mở có evidence | `record --kind intervention --question` (cap 2 câu + frequency cap) | `EXISTS` (cưỡng chế, doc chỉ khuyên) | `Local` |
| Relay owner decision | mailbox + `pending` + `mark-delivered` | `EXISTS` (nhưng chưa có consumer) | `Local` |
| Báo Human bias/risk, escalation path | `--kind escalation` / `--kind urgent` + desktop notify | `EXISTS` | `Local` |
| Notebook có causal context | `observations.jsonl` + `--note` free text | `PARTIAL` — 7-field shape không được CLI cưỡng chế | `Local` |
| Đề xuất patch profile/protocol | `bee-evolving` (gated, bee repo only) | `PARTIAL` — khác shape, không do supervisor kích hoạt | `Local` |
| Tạo Lead mới + bounded handoff | `bee state handoff write/adopt` tồn tại, nhưng không có đường supervisor-authorized | `NEW` | `Local` |
| Sparse, event-driven, không polling | `away`/`back` + WakeReport + consent-sweep | `EXISTS` (giàu hơn doc) | `Local` |
| Read-only, không sửa code (§8.17) | CLI tự chặn: "dispatches nothing, approves nothing" | `EXISTS` | `Local` |
| Supervisor dùng model rẻ | `.bee/config.json:57` role `supervisor` = haiku | `EXISTS` | `Local` |
| **Có agent thực sự ngồi ghế đó** | không skill, không loop, store rỗng | **`NEW` — đây là toàn bộ khoảng trống** | `Local` |
| Paseo là control plane | paseo đã gỡ hết khỏi waggledance | `CONFLICT` | `Local` (`57f3193`) |
| Tên "supervisor" | 3 nghĩa không liên quan trong cùng stack | `CONFLICT` (nguy cơ nhầm lẫn) | `Local` |

### Cross-cutting sweep

Wiring nằm ngoài thư mục feature, đã kiểm:

- `.bee/supervisor/` ở **control root, dùng chung mọi worktree** — không per-worktree.
  Chưa tồn tại.
- `.bee/config.json`: key `supervisor.notify` (default on), `supervisor.consent`
  (fail closed), và role `supervisor` ở `models.claude.roles`.
- **Turn boundary**: mailbox chỉ có nghĩa nếu session đọc `pending` mỗi lượt.
  Không tìm thấy hook nào làm việc đó — `rg "supervisor" .claude/` ra rỗng.
- Desktop notification path (`--kind urgent`) — fire-and-forget, chưa kiểm được
  vì chưa có row nào.
- Phía waggledance: `waggledance_ask_state` / `waggledance_runs` là fleet-read mà
  một observer sẽ cần. Reaper và notify watcher là task daemon, **không** liên quan
  tới governance plane.

### Inference

- `Inference` — bee supervisor được thiết kế bởi người đã đọc chính mô hình này
  hoặc một mô hình rất gần: frequency cap ↔ §8.17, silence-là-record ↔ §7.6,
  band hai phía ↔ §8.10 polling debt + "supervisor im lặng cũng là hỏng". Không
  chứng minh được từ record nào, chỉ là độ khớp quá cao để là ngẫu nhiên.
- `Inference` — lý do ghế trống có thể đơn giản là bee-herding's control loop đã
  hút mất phần "vòng lặp lạnh", và supervisor bị bỏ lại như một surface chưa dùng.
  Chưa kiểm chứng.
- `Inference` — món có giá trị nhất trong doc mà bee **chưa** có, không phải cơ
  chế nào cả, mà là **`WORKSPACE_PROTOCOL.md`** (§6): lớp giữa profile và task
  prompt, giữ chiến thuật riêng của từng repo, có version. bee trộn lớp này vào
  AGENTS.md + skill.

## Risks, Unknowns, Follow-Ups

- **Rủi ro chính nếu dựng ghế supervisor:** §8.11 ceremony capture và §8.14
  attention dilution. Một observer chạy đều đặn mà không có event thật sẽ tạo
  record rỗng và làm loãng chú ý — chính bee đã lường trước bằng band hai phía,
  nhưng band chỉ đo được khi đã có sample.
- **Rủi ro tên gọi:** ba "supervisor" trong một stack. Bất kỳ tài liệu nào từ đây
  về sau nói "supervisor" mà không gắn tiền tố sẽ mơ hồ.
- **Unknown:** ai (nếu có) đáng lẽ phải gọi `bee supervisor pending` ở turn
  boundary. Nếu không ai, mailbox là write-only.
- **Unknown:** doc nhãn `[SYNTHESIS]` cho phần profile — không rõ Demonthorn có
  thực sự nói "Supervisor tạo Lead mới" hay đó là suy diễn của người viết doc.
- **Câu hỏi mở cho người dùng:** đây là đọc-để-hiểu, hay là bước đầu muốn thực sự
  cho supervisor chạy trong repo này?

## Source Pack

- **Local files read:** `crates/waggledance/src/supervisor.rs`,
  `crates/waggledance/src/reaper.rs` (§1-130), `crates/waggledance/src/main.rs`
  (§160-347), `crates/waggledance-core/src/config.rs` (§60-110),
  `~/.waggledance/config.toml`, `~/.claude/skills/waggledance-supervisor/SKILL.md`,
  `.bee/config.json`, `.claude/skills/` (listing + grep)
- **Local commands:** `bee supervisor --help`, `bee supervisor presence --json`,
  `ls .bee/supervisor/`, `ps aux | grep -iE "waggledance|herdr"`
- **Source distilled:** `refs/slp/paseo-pi-team/docs/demonthorn-agent-orchestration-deep-dive.md`
  — §Executive summary, §4.1-4.3, §5.5, §7.6-7.8, §8.10-8.17, §9, §10
- **Upstream repos / docs pages:** none — no web research run (capability gap,
  claims above degraded to `Inference` where affected)
