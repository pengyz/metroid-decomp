---
type: reference
---

# Mesen2 场景 trace 确定性（A 级实证）

同一场景 DSL 同机双跑，`frames.jsonl`（帧号+watch 状态向量）**逐字节一致**
（boot-title 300 帧 ×2，sha256 `4655a986264d0104…`，2026-10-14）。

意义：L3 manifest 只存 sha256 摘要即可充当确定性基线——每次门禁运行重采集并校验摘要，
持续复验确定性（`tools/diff_traces.py`）。注意范围：仅证 watch 状态向量层面；
captures.jsonl（含产物路径）与截图 PNG 不在确定性承诺内。
