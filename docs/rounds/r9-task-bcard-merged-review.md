# R9 对抗复核 verdict——B 卡全案（20706c93 TDD 红测 + 765f1352 修径 a + for_rebase 重基）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-03
- **verdict：pass**（附滥用面余量注记 + F 卡姊妹面建议）

## 1. 修径 a 正确性 —— PASS

- **双相位模型**（rust_screenshot.rs 实读）：capture 帧 N = tick(N-1) 双相位——
  png/ppu.hex=on_spin_exit 缝快照（RenderState）+**OAM 收口活体**；ram.hex=收口
  活体（$1A=00 判别位）；缺缝回退活体（span/title 续段不采——冻结窗内等价注）。
- **3 元组终形审**：`seam_snap: Option<(usize, RenderState, Vec<u8>)>`——
  reflog/amend 考古实证：前变体 e219a70b 为 **2 元组**（snap:(tick,RenderState)，
  OAM 渲染时现取=缝相位错——探针定谳 oam@缝 397px/rows1128 397px），amend 修为
  3 元组（OAM 缝时 to_vec 同捕）+**探针定谳 rs@缝+oam@收口 vs emu f1128 逐像素
  0 差**——终形正确 ✓。
- **walkfcprobe 双实测锚**：锚 A cap(N)≡行(N) RAM 面（FC/ScrollX/SamusX 0 差）、
  锚 B dx 三点（0/332/674）崩落——findings §2/§6a 对读一致；capture_seam TDD
  **红→绿实证**（红态实录：capture(450)=(FC180,0,128) vs 行=(FC179,0,128)+1 面；
  本复跑 `capture_seam` 测绿 1/1）✓。

## 2. 基线重基合规 —— PASS

- **kind=rebase+evidence 双留痕**：l4_ratchet_baseline.json history 终条实录
  `{kind:"rebase", evidence:"walkfcprobe 双实测锚…466 测 0 败；矩阵 fresh 双腿
  match 57/diff 51/blocked 0", reason:"采集语义重基（修径 a 截取点双相位）：2 格
  翻面=morphball f1128/f1200 旧 match=旧采样缝掩蔽假 match…"}` ✓
  （注：rebase history 在 ratchet baseline 非 status_quo——初查位置错，已定位）。
- **2 翻格归因核**：「旧 match=旧采样缝掩蔽假 match（4 周期别名假合）」——与
  B1 定谳（capture 缝相位模型）同源一致；重基=真值重基**非容忍账**（reason/
  exit 未走容忍语义）✓——与 A-full「真值首锚」设计衔接。

## 3. for_rebase 门禁路径 —— PASS（附余量注记）

- **阻断零弱化**：gate 检查侧调用 `l4_ratchet_violations(baseline, matrix)`
  （gates/cli.py:1268）**不传 for_rebase**（默认 False）——check 时 match 缩水
  仍拒；rebase 仅 tools/baseline_update.py l4 --rebase 单写入口。
- **selftest 113/113 复跑通过**（含 F③ ≤2 次放行正向用例）✓。
- **强制留痕**：`rebase and not evidence ⇒ 拒`（baseline_update.py:57）；
  history kind=rebase 审计可见 ✓。
- **滥用面余量注记（非阻塞）**：evidence 为自由文本、无外部校验——理论上仍可
  以编造 reason 吞回退。现有防线（evidence 强制+history+gate 侧不变）对单
  operator 流已够；若未来多 agent 并发写基线（goal 循环 F 卡场景），建议加
  ②history 逐条 diff 摘要（重基前后 match 集清单）机械化对读——登记 F 卡
  姊妹面，不阻塞本审。

## 4. 复跑 —— PASS

- `cargo test --workspace`：**466 passed / 0 failed**（含 capture_seam 新测）✓。
- L4 fresh run-all：**match 57 / diff 51 / blocked 0（57/108）**——与重基 history
  声明值逐项一致 ✓。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**；selftest 113/113 ✓。

## 5. 并发事故记录 —— PASS（附 F 卡姊妹面建议）

- 事故与落档：stash 丢失（merge 吞 staged 变更）经 git fsck 救回 58a5a334
  （merge commit 在案）；kb 已固化 **gotcha-merge-stash-recovery**（含共享树
  禁 stash 第七案规）✓——机制面已落。
- **建议（F 卡姊妹面）**：goal 循环多会话并发（两轮坏读数污染）属协作纪律面
  ——kb 已有 gotcha-concurrent-mesen-output-corruption；建议 F 卡机械闸设计时
  并入「采集类工件 run 隔离戳（run id 写入产物名/matrix）」使并发坏读数可检出
  而非仅靠人查——登记不阻塞。

## 裁定

| 点 | 结论 |
|---|---|
| 1 修径 a（双相位模型+3 元组终形+探针 0 差锚+红→绿） | pass |
| 2 重基合规（kind=rebase+evidence 留痕+真值重基非容忍账） | pass |
| 3 for_rebase（gate 零弱化+selftest 113+滥用余量注记） | pass |
| 4 复跑（466/0；57/51/0；gates 13/0） | pass |
| 5 并发事故（kb 已落+F 卡姊妹面建议） | pass |

**verdict = pass**。

Evidence: rust_screenshot.rs 双相位/3 元组实读+e219a70b→765f1352 amend diff；
reflog（commit (amend) 链）；l4_ratchet_baseline.json history rebase 条实录；
gates/cli.py :1268 调用形+baseline_update.py :57 evidence 强制；selftest 113/113；
case-killenemy/probe 矩阵 fresh（57/51/0=108）；cargo test（466/0）；
gates check（PASS=13 FAIL=0）；docs/knowledge gotcha-merge-stash-recovery；
merge commit 58a5a334。
