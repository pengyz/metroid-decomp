# R9 对抗复核 verdict——门禁债清理三 commit（69e3689e/4d884683/15b46adf）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-02
- **verdict：pass**

## 1. zp span 根因核 —— PASS

- **旧污染实证**（git show 69e3689e 前形）：EnsExtra 附加（`$6AF4-$6B73` +128B）
  写在 `readRange` 内部 ⇒ **全部 range 读被污染**——zp watch 256B+128=**384B
  （0x180）vs manifest 256B** 不符实锤。
- **修形核**（现 file 实读）：附加收进新 `readRangeWithEnsExtra`，仅 dumpRam
  （$0000-07FF，:137）使用；`readRange` 恢复纯净（:252/:553/:554/:585/:589 各
  消费点不再被附加）——w[zp]=256B 归正 ✓；wram watch 6000-7FFF 本身已覆 $6AF4
  （冗余无害注在案）。

## 2. morph-ball sha 重锚正当性 —— PASS

- 演进链两 commit 在库：**03d3562a**（10-31 能力链移植）/ **01a572d4**（10-31
  镜像修）——goal 轮合法演进 ⇒ scenario_sha256 a3c140ac→c6598bf6 重锚。
- 留痕合规：l3_manifest.json `scenario_sha256_note`（旧→新 sha+理由日期）+
  三场景 `determinism_evidence` 重采链（旧 ddb2d768→e5fe901e / beeebb5b→3b24f797 /
  5e411098→dc651561，zp 归正因注）——非静默改锚 ✓。

## 3. 账登记合规（抽查 5 项逐一） —— PASS

- 逐条判定面：账内「渲染腿缺口」类条目 **46**（新 32 sc-* + 旧 13 case-001 族+
  结构行），observed 签名逐条带 shots/ 金面文件名。
- **抽查 5 项（seed=7 随机）独立核**：`sc-first-room-full_f02130_shoot-enemy`/
  `case-001…f00950_combo-action`/`sc-first-room-full_f02780_after-pickup`/
  `sc-shoot-kill_f01100_shooting`/`case-001…f00650_after-left`——**5/5 金面在
  shots/ 存在 ∧ analysis/render-shots/ 无同名**（非批量盲登）✓。
- observed 精确等值对齐：runner [FAIL]/[R-FAIL] 行 rest-vs-observed 逐条机械
  对账（gates/cli.py 现状基线读取点）——**l4 闸 PASS=机械对账通过**（count==len
  由闸自证）。

## 4. 复跑 —— PASS

- `cargo test --workspace`：**463 passed / 0 failed**（与声明一致）。
- L4 fresh run-all：**match 57 / 103**（≥53 无回退；goal 轮新 case 并入分母）。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**（trace-diff/visual-diff
  账内容忍显式打印）。

## 5. 分面纪律 —— PASS

- 69e3689e=tools 单面 / 4d884683=oracle manifest 单面 / 15b46adf=data/status 单面
  ——三 commit 一面一改，符合 AGENTS「门禁/基线改动单独 commit」纪律。

## 裁定

| 点 | 结论 |
|---|---|
| 1 zp span 根因（384B 污染→收进 dumpRam 专面） | pass |
| 2 morph-ball 重锚（演进链+留痕） | pass |
| 3 账登记（46 条类；抽查 5/5 金面在+渲染腿缺；闸机械对账） | pass |
| 4 复跑（463/0；57/103；CHECK: PASS） | pass |
| 5 分面纪律 | pass |

**verdict = pass**。

Evidence: tools/scenario_capture.lua :99-137（readRange/readRangeWithEnsExtra/
消费点实读）；oracle/traces/l3_manifest.json（sha 留痕 diff）；data/status/
l4_status_quo_baseline.json（46 类条目+seed=7 抽查 5 项脚本实跑）；git log
（03d3562a/01a572d4 演进链）；shots/rust L4（fresh 57/103）；cargo test（463/0）；
gates check（PASS=13 FAIL=0）。
