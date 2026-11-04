# animI 走表轨迹域·collide f800 animI 5 步滞后——偏离点定位（卡 A）

> 战役：animI/走表轨迹域。卡 A=asm 逐行读证 SetSamusAnim 族 + animResetIndex 写源
> vs rust 初值路径，定位 collide f800 animI 5 步滞后（emu 0D/rust 08）的偏离点。
> 结论分级见 §5。纪律=AGENTS.md（证据 A/B/C 分级 + 禁试错式修改 + 证伪更正）。

## 1. 症状（A 级实录——两腿 capture 点 RAM 直读）

collide f800（`case-collide-probe_f00800_l240.ram.hex`，两腿全 2KB RAM hexdump 直读）：

| 字段 | 地址 | emu（oracle 腿） | rust（shots_rust 腿） | 判 |
|---|---|---|---|---|
| animFrame | $0303 | **0x10** | 0x08 | 分歧 |
| animDelay | $0304 | **0x02** | 0x01 | 分歧（相位差 1） |
| animResetIndex | $0305 | **0x0C** | 0x0C | **一致** |
| animIndex | $0306 | **0x0D** | 0x08 | 分歧（rust 落后一走表步） |

**关键**：animResetIndex 两腿一致（0x0C=SamusJump）⇒**写源值正确**，非 reset 写值错。
分歧在 **animIndex/animFrame/animDelay**（走表推进态）。

## 2. 走表时序（全 capture 点两腿对表——A 级）

| frame | emu(frm,dly,rst,idx) | rust(frm,dly,rst,idx) | 判 |
|---|---|---|---|
| f420 | (00,00,00,00) | (00,00,00,00) | 同 |
| f540 | (07,00,04,05) | (07,00,04,05) | 同 |
| f620 | (07,00,04,05) | (07,00,04,05) | 同 |
| f700 | (07,00,04,05) | (07,00,04,05) | 同 |
| **f800** | (10,02,**0c**,0d) | (08,01,**0c**,08) | **首分歧** |
| f890 | (08,00,07,07) | (08,00,07,08) | 分歧（idx 差 1） |
| f900 | (08,02,00,08) | (05,01,00,03) | 分歧 |

anim 常量（handler.rs:125-134）：RUN=0x00/FRONT=0x04/STAND=0x07/JUMP=0x0C/SALTO=0x0E。
collide 动画径（f700→f800）：**FRONT(0x04)→STAND(0x07)→JUMP(0x0C)**。

## 3. 偏离点（A 级定位）

f800 时：**emu 已收敛到 JUMP 走表**（animIndex=0x0D=JUMP 稳态（reset=0x0C 单帧表
table[0x0C]=0x10→idx←0x0D→table[0x0D]=FF→wrap 回 0x0C→稳态 0x0D）；frame=0x10=JUMP 帧）。
**rust 仍显示 STAND 帧**（frame=0x08=table[0x07]=STAND 帧；animIndex=0x08=STAND 走表位
（table[0x08]=FF，下一步 wrap））。即 **rust 落后 emu 一走表推进步**（STAND→JUMP 的
FF-wrap 尚未发生）。

**偏离点 = animResetIndex-only 写后的 animIndex 收敛时机差**：
- asm 走表族（SamusRun samL01/02/03/04 + falling 臂）切动画时**只写 animResetIndex**
  （`sta Samus.animResetIndex`，不写 animIndex）——animIndex 靠 FF-wrap 收敛到新 reset。
- rust 同形（handler.rs:166/247 等**只写 reset**）——**忠实**。
- 但 **emu 在 f800 已 wrap 收敛（JUMP 0x0D），rust 尚未 wrap（STAND 0x08）**——收敛时机
  差 **1 走表步（≈2 帧 @ A=2 空中节律）**，伴随 animDelay 相位差 1（emu=0x02/rust=0x01）。

## 4. SetSamusAnim 族 asm 语义（A 级逐行读证）

| 件 | bank:addr | 写面 | 备注 |
|---|---|---|---|
| SetSamusAnim | 07:CF6B | reset←A；（fallthrough）index←A；delay←0 | 双写 reset+index |
| SetSamusNextAnim | 07:CF6E | index←A；delay←0 | **只写 index**（不写 reset） |
| InitObjectXSlotAnimIndex | 07:D2FA | reset,x←A；（D2FD）index,x←A；delay,x←0 | 对象槽索引版 |
| SetSamusJump | 07:CFC3 | reset←Jump；**index←Jump-1**（dey=JumpTransition）；delay←4 | jump 起手带过渡帧 |
| SetSamusData_3Frame | 07:CD6B | A=#$03→UpdateObjAnim | 地面走表节律（3 帧） |
| SetSamusData（空中 samL06） | 07:CD16 | A=#$02→UpdateObjAnim | 空中走表节律（2 帧） |
| UpdateObjAnim | 07:DC8F | animDelay 倒数→0 时重载 A + animIndex 推进；FF→wrap 回 reset | 走表核 |

rust 对应件（忠实面 ✓）：
- `set_samus_animation`（animation.rs:47）=[$0305←A/$0306←A/$0304←0] ✓（SetSamusAnim 全形）
- `set_samus_jump`（handler.rs:447）=[reset←0x0C/index←0x0B/delay←4] ✓（SetSamusJump dey 形）
- `update_obj_anim`（physics.rs:892）走表核 ✓（FF-wrap 回 reset，PageIndex 基址）
- `set_samus_data`（physics.rs:817）→`update_obj_anim(mem, mem.regs.a)`（A=节律重载值）
- 节律 A 值：空中共 `mem.regs.a=2`（handler.rs:265，asm samL06 `lda #$02` ✓）/地面共
  `mem.regs.a=3`（handler.rs:329 等，asm CD6B `lda #$03` ✓）。

## 5. 结论分级

- **已证实（A 级）**：
  1. animResetIndex 写源值正确（f800 两腿均 0x0C=JUMP）——非 reset 写值错。
  2. SetSamusAnim 族 asm 语义（§4 表）+rust 对应件忠实（set_samus_animation/
     set_samus_jump/update_obj_anim/set_samus_data 节律 A 值全对表）。
  3. 偏离点=animResetIndex-only 写后 **animIndex 收敛时机差 1 走表步**（emu f800 已
     wrap 到 JUMP 稳态 0x0D，rust 尚在 STAND 位 0x08）+animDelay 相位差 1（0x02/0x01）。
  4. collide 动画径=FRONT(0x04)→STAND(0x07)→JUMP(0x0C)（f700→f800）。
- **假设（待证）**：
  - 收敛时机差 1 步的**根**=①animDelay 走表节律相位累积差（r14「AD 周期别名」残余）
    或②切动画触发时机差（physics：SamusAccelY/falling 判定帧相）——**需逐帧 trace 定帧**
    才能钉单一修点（禁试错式修改——AGENTS.md §3.5）。二者均表现为 ~1-2 帧偏移。

## 6. 下一步（卡 B 候选）

1. 逐帧 trace（DSH_SPRITE_TRACE 桩已备：`[uoa]/[anim]/[ssd]` 打 animIndex/reset/delay/A
   逐帧）对 collide f700-f800 两腿逐帧对表——定 STAND→JUMP 切换的**精确分歧帧**。
2. 依分歧帧定根（①节律相位 or ②切换触发时机）→TDD 红（钉收敛时机）→修→绿。
3. 判据：collide f800 animIndex 两腿 0 差（+全族 l4 无回退）。
