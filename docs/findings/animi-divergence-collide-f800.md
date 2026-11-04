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


## 7. 卡 B·逐帧 trace 定根（2026-11-04）——近因①证实 + 深根=anim-set 路径/移动态

【双腿逐帧实录（A 级）】emu watch_range 0305-0306 逐帧 + rust DSH_SPRITE_TRACE（[uoa]/[anim]/[ssd]）。
- **EMU**：f778-796 FRONT(reset=04,idx=05 稳)→**f797 (reset=00,idx=08)**→**f798 JUMP(0C,0D 稳态)**。
  f797→f798：idx 0x08→0x0D=FF-wrap **即时**收 wrap（animDelay=0⇒推进即刻，同帧切同帧 wrap）。
- **RUST**：n=785 FRONT(04,04)→**n=787 STAND(07,07) a_in=4**→**n=788 JUMP(0C,idx=08) ad=4 a_in=2**
  →n=789 ad=3→n=790 ad=2→…→n≈792 ad=0 才 wrap 收敛。idx=0x08（STAND 位滞）阻 ~4 帧。

【近因裁定=假设① animDelay 走表节律相位（证实）】切换点（reset←JUMP）animDelay 值：
**emu=0**（推进帧，idx 即 wrap 到 JUMP 0x0D）vs **rust=4**（STAND a_in=4 重载继承（asm LCC6E
`lda #$04` 忠实）阻 wrap）。收敛 wrap emu@f798 vs rust@n≈792（迟 ~4 帧）。

【深根=anim-set 路径/移动态分歧（A 级）】切换点**前一 anim**不同：
- emu：**RUN(0x00)**@f797→samL04（`cmp animReset==Run`⇒`sta animReset←Jump`）→JUMP。
- rust：**STAND(0x07)**@n=787→falling 径（asm 2868 `sta animReset←Jump`（falling+disp<0x18））→JUMP。
即 emu 走 samL04（RUN→JUMP），rust 走 falling（STAND→JUMP）——**前 anim 移动态不同**（emu 判
Samus 在跑 RUN，rust 判停 STAND）。碰撞后移动态（跑 vs 停）分歧⇒anim-set 路径分叉⇒animDelay
相位（RUN/STAND 节律不同）⇒收敛时机差。触发径本身（falling+disp<0x18（handler.rs:224≈asm
2864-2870））忠实；samL04（handler.rs:246）忠实。**分叉源=前 anim 移动态（RUN vs STAND）**。

【结论分级】
- 已证实（A 级）：近因=animDelay 相位（emu 0/rust 4 阻 wrap）；深根=anim-set 路径分叉（emu
  RUN→samL04 vs rust STAND→falling）=**移动态分歧**；触发径/samL04/LCC6E 节律全忠实。
- 待证：移动态分歧（RUN vs STAND）根=碰撞/physics（Samus 移速/位移判）——属 physics 域，
  非 anim 域局部可修（AGENTS.md §4 定位层：跨层→physics）。

【修径】
- anim 域局部（近因）：切换点 reset←JUMP 时 idx 已在 FF（旧 anim 末）⇒即 wrap（不阻于
  animDelay）——但 asm 走表核 animDelay=0 才推进（忠实）⇒此修**非 asm 忠实**（需 emu 反证
  emu 亦仅巧合 ad=0）。
- physics 域（深根）：移动态（碰撞后跑/停）对齐 emu——Samus 速/位移/碰撞反应径 asm 对读。
判据：collide f800 animIndex 两腿 0 差（idx=0x0D）+l4 无回退+443 测全绿。
