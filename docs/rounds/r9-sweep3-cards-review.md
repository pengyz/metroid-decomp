# R9 小件清扫三格+挂起卡落账 — 独立对抗复核（verifier，证伪立场）

- 复核对象：commit 链 89b72285 → e1609e28 → dc2504e3 → c68cfaff → 74a36f84（HEAD）
- 复核人：verifier agent（3e46d0f6），不为实施方立场辩护
- 日期：2026-11-09

## Verdict: **pass**

## 逐点核验（证据均为本人重导/重跑，非采信 porter 陈述）

### 1. git diff 逐 commit 审 — 无夹带/越权/弱化

`git diff --stat 89b72285^..74a36f84` 全范围仅 6 文件：
- `tools/scenario_capture.lua`（+24/-7，仅 harness 生命周期）
- `crates/game-core/src/bin/rust_screenshot.rs`（+68/-4，采集相位）
- `crates/game-core/src/frame/room.rs`（+51/-6，写径修正+新测试）
- `data/status/l4_ratchet_baseline.json`（103→108，只升不降：5 个 diff 格全转 match，无 match→diff 回退）
- `docs/findings/morphball-trajectory-root.md`（§63/§64 追加）
- `docs/pending-cards.md`（新建，34 行）
- **gates/ 目录零改动、白名单零改动（matrix 白名单条目=0）、无阈值/容差引入。**

### 2. harness 修复语义核（e1609e28）— 成立

- writeDoneAndStop 三条路径全覆盖（scenario_capture.lua:217-218 末帧兑现收口 / :826 frames+1 兜底收口 / :971 无 pendingShot 即时收口）；doneWritten 幂等守卫（:176）。
- close 移入收口（:186-188）后，采样/截取均在文件存活期内完成；原 endFrame 预关点（现 :857 注释）已删。
- pcall 取舍：SAMPLE-ERR 显式留痕（:766-770 log），采样错误不再中断截取兑现——是文档化取舍。**观察项（非阻塞）**：SAMPLE-ERR 目前仅落 lua.log，无机械检查；建议后续把「l4 run 的 lua.log 含 SAMPLE-ERR ⇒ FAIL/flaky」纳入工具面，防采样回归静默产半迹。
- 修径 C 协议（DONE-DEFER→缝兑现→收口）恢复并留 dated correction（:819-822）。

### 3. OAM 恒取缝克隆核（dc2504e3）— 成立

- 缺缝路径：`capture_inputs(None, …)` ⇒ rs+OAM 活体回退（rust_screenshot.rs:562-570，双测试 :640/:652 钉死）——异常路径行为明确，非静默。
- 与修径 a 双相位一致性：seam_snap 在 tick_frame_with 的 on_spin_exit 内克隆 `mem.ram[0x200..0x300]`（rust_screenshot.rs:474-478），发射点 `*k == frame-1` 校验（:443-446）——OAM 克隆与 rs 快照同缝（post-NMI/pre-work），与 emu spin-exit-seam 截取同相位。
- 语义自洽（本人推演）：缝时刻 SpriteRAM=$0200-2FF 是上一帧工作产物，NMI 只 DMA 不写 SpriteRAM（NMI :233-249 取证），故缝克隆=emu 缝截图 OAM 内容；f650 全 0xF0 vs emu png 有精灵的 dump 相位实录佐证两相位差异非本判据杜撰。
- dated correction 链完整：rust_screenshot.rs:538-554（①恒取收口→②is_end/span 证伪→③fallback 根因）+ findings §63 同步落账。

### 4. AttribTableWrite asm 对读（亲自逐行）— porter 主张成立

refs/m1disasm/SRC/prg7_engine.asm：
- AttribTableWrite $EA13（:8557-8566）：`lda RoomNumber; and #$0F; inc RoomNumber; jsr JumpEngine` — 表 [ExitSub, WritePPUAttribTbl(row1), ExitSub, WritePPUAttribTbl(row3), RoomFinished]。**inc 在 dispatch 前**——WritePPUAttribTbl 入口时 RoomNumber 已自进：row1 入口 $F2、row3 入口 $F4。
- WritePPUAttribTbl $E5E2（:7710-7746）：`ldx #$C0`；`cmp #$F2; beq LE5EC`（**$F2 即 row1 后态 ⇒ 保留 $C0**）；否则 `ldx #$E0`（$F4 即 row3 后态 ⇒ $E0）。porter 的 row1→$C0 / row3→$E0 映射正确——易错点在 inc 时序，porter 对了。
- GetNameAddrs $E564（:7615-7623）：`lda PPUAddrs,y`（PPUAddrs=$20/$2C，:7607-7610）+ 调用侧 `ora #$03` ⇒ $23/$2F ✓；`txa; ora #$03` ⇒ RoomRAM 源 $63/$67 ✓。
- `lda #$20` 32B 整行 ×2（:7737-7743）= 全 64B ✓。
- 旧径算式验证（本人算）：`0x23C0+(3-1)*0x20 = 0x23C0+0x40 = 0x2400` ✓ —— row3 确实落 NT B 瓦面 row0；8B 直写 + src+0x40 溢出属性区（$x3C0+$40=$x400 越界）主张成立。
- rust 新径（room.rs:641-653）：row_off=(row-1)*0x10（$00/$20=$C0/$E0 低字节 ✓）、nt_hi=(hi>>2)&1?$2F:$23（hi=$60/$64 ⇒ bit2 ⇒ $23/$2F ✓，与 select_room_ram :602 hi 构造自洽）、src=((hi+3)<<8)|(0xC0+row_off) ✓、`dst+k`（修掉旧 `dst|k` 位或 bug）✓、32B ✓。测试 attrib_table_write_nt3_targets_mirror_attr_rows 钉 $2FC0/$2FE0 + NT B 瓦面不被污染 + RoomNumber 自进 F3→F4 ✓。

### 5. 基线重基正当性（74a36f84）— 成立

- **金面=真缝相实证**：四案端采集 lua.log（本人重查 analysis/traces/）：crawlprobe f960 / walkfcprobe f1400 / fcprobe f460 / morphball f1700 末帧均 `SHOT-SEAM … reason=spin-exit-seam`，**零 DONE-FALLBACK、零 SAMPLE-ERR**——非工具 bug 二次产物。
- 全矩阵 fresh 双腿复跑（matrix.json provenance 全 `fresh/fresh`），非缓存旧面。
- 基线 diff 只含 diff→match 五格（950:combo-action/1000:neighbor-room/1150:end/960:end 及 walkfcprobe 面），无任何 match→diff。

### 6. 独立复跑（A 级实录，本机重跑）

- `cargo test --workspace`：**exit 0，全绿**（game_core 463 + rust_screenshot 5 + 集成 4 + game_data 27 + game_platform 4 + …，ratchet 计 507）。
- `python3 gates/cli.py check`：**SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2，EXIT=0**。两 TOLERATED（trace-diff samus-move3 缓存缺损、visual-diff 旧定义采集面）均为 2026-10-26/11-02 既有登记，非本批引入。
- `python3 tools/l4_diff_run.py run-all`（独立重跑 13 案全 108 格，双腿 fresh）：**match 108 · diff 0 · blocked 0 · pass 108/108 ✅**。
- `python3 gates/cli.py selftest`：**118/118 通过**（C1 崩溃残留拒裁、账外照常 FAIL、F①②③、G①②③ 等负控全过）。

### 7. 防弱化审计

- 本批零 gates/ 改动、零白名单条目、零阈值/容差；ratchet 分母 108 只增、match 只增不降。
- selftest 全绿（门禁判定逻辑未被削弱）。

## 非阻塞观察项（留给后续卡，不构成 needs_revision）

1. **ratchet:rust_test_count PENDING（503→507）**：check 输出显式 WARN，需独立 commit `baseline update --reason` 上调（新增 4 测试）。未随本批更新=卫生欠账，非门禁失败。
2. **SAMPLE-ERR 无机械检查**（见 §2）：pcall 后采样失败仅留痕不拦截；建议后续把 SAMPLE-ERR 存在性纳入 l4 工具裁定。

## Confidence: 93
