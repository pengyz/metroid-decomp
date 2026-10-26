# L4 对拍成果 · 独立对抗性复核（证伪立场）

- 复核对象：`tools/l4_diff_run.py` + `crates/game-core/src/bin/rust_screenshot.rs` + case-001 对拍结论
- 复核角色：remake-verifier（差分验证员，**证伪立场**，非背书）
- 日期：2026-08-26
- **verdict：PASS**（对拍工具真实、数字准确、口径诚实；一处根因被证伪后**更锐化**——见 F5）

> 注：原定 subagent 复核因上下文超限中断（未落盘）；本文件由 lead 以 verifier 证伪立场
> 独立重导证据补记。每条结论均标注可复现证据（命令/文件:行/像素数字）。

## F1. 对拍是否「两条独立执行径」？→ ✅ 成立
独立重算（`python3 -c` + PIL + sha256）：
```
f00420 rust_sha=48c2da4b93e1 emu_sha=8cbf4a35ef74  独立（不同文件）
f00720 rust_sha=48c2da4b93e1 emu_sha=596bb5b093db  独立
f00850 rust_sha=e5f238286ef1 emu_sha=4142e57bae53  独立
f01000 rust_sha=ec0fa1541891 emu_sha=c4d560c4baed  独立
```
- Rust PNG 与 emu PNG **sha 全不同**（非复制、非同源）。
- 代码证据：`rust_screenshot.rs` 走 `FrameEngine::tick_frame`→`render_frame`（真 RenderState/
  调色板/OAM 影子 `ram[0x200..0x300]`）；emu 走 `scenario_run.py`→Mesen2 执行 ROM。
- **非「模拟器 vs 模拟器」**——两条执行径独立。

## F2. Rust 帧是否真实（非黑图/非模拟数据）？→ ✅ 成立
```
f420 rust非黑px=12759 rust独色=4   f850 rust非黑px=11517 rust独色=7   f1000 rust非黑px=11225 rust独色=6
```
- Rust 帧有真实调色板色（4–7 独色、万余非黑像素），**非全黑、非 fill_render_state 模拟数据**。
- 这推翻了旧版「黑图冒充渲染帧」问题；现版是真渲染。

## F3. diff 数字是否准确？→ ✅ 成立（逐像素重算复现）
```
f420 diff=0    f720 diff=282    f850 diff=2385    f1000 diff=17800
```
与实施方声称**逐值相符**。numpy 逐像素零容差重算，无四舍五入/近似。

## F4. FIRE=A vs B 考证？→ ✅ 成立（A 级 asm）
asm `SamusJump_CheckFire ; 07:D09C`：
```
lda Joy1Change / ora Joy1Retrig / asl / bpl RTS_X028   ; exit if FIRE not pressed
```
`asl` 把 bit6 移入 bit7，`bpl` 测 bit7 ⇒ 测的是**原 bit6 = BUTTON_B(0x40)** ⇒ **FIRE=B**。
`CheckBombLaunch ; 07:D150` 同形（`asl/bpl` 测 bit6）复证。
对照 `crates/game-core/src/samus/handler.rs:613`：`if fire & A != 0`（A=0x80）——**错判为 A**。
（JUMP 判 `JOY1_CHANGE & A` 正确；FIRE 误用 A 致 `press a`(跳跃) 走 fire 路径引爆 D1EE。）
**结论：实施方主张正确，asm A 级证据坐实。**

## F5. 「f720=Rust 不画萨姆斯精灵」根因？→ ⚠️ 方向对，但**被证伪后更锐化**
证伪发现：**`rust_sha(f420) == rust_sha(f720)`（48c2da4b93e1，逐字节相同）**！
- emu f420(8cbf…) ≠ emu f720(596b…)（emu 在此区间推进——萨姆斯 beam-in/就绪动画）；
- rust f420 == rust f720（**Rust 在无输入 idle 帧完全冻结**，渲染逐字节不变）。
⇒ **更深层根因**：Rust 游戏逻辑在 idle 帧（f420→f720）**不推进游戏状态**（萨姆斯 intro/
就绪动画未实现/未跑），故整帧冻结；「不画萨姆斯精灵」只是表症。移动帧(f850/f1000)因输入
才推进。**实施方『Rust 不画萨姆斯』描述的根因需修正为『Rust idle 帧游戏态冻结、缺萨姆斯
intro 动画推进』**（同为游戏逻辑移植缺口，非对拍工具问题）。

## F6. 有无假声称？→ ✅ 无
- 4 个缺口采集点诚实标 `⛔缺口 blocked: unported @ bank07:D1EE`，**未**说成「一致」。
- 报告 `shots_rust/*.l4report.json` 区分 match/diff/blocked 三态；总评「❌ 未全一致」诚实。
- 无「模拟器 vs 模拟器」冒充、无黑图冒充、无 hash 近似糊弄。

## 总评
**PASS**。L4 对拍工具真实可用（两条独立执行径、真渲染帧、逐像素零容差），case-001 数字准确，
口径诚实无假声称。实施方自身的游戏逻辑缺口（idle 帧态冻结/萨姆斯 intro、FireSamusProjectile
未移植、FIRE 误判 A）均如实暴露，属移植缺口而非工具缺陷。F5 根因已锐化更正。

**可复现命令**：
```bash
cargo build -p game-core --bin rust_screenshot
python3 tools/l4_diff_run.py run case-001-game-start-samus-interaction --timeout 300
python3 -c "from PIL import Image;import numpy as np;..."   # 逐像素重算（见 F3）
sed -n '/^SamusJump_CheckFire/,/notAimingUp/p' refs/m1disasm/SRC/prg7_engine.asm  # F4 asm
```
