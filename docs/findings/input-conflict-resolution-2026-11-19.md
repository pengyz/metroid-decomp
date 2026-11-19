# 输入注入器对向键重复消解（oracle 腿缺陷）

**结论**：`tools/scenario_capture.lua` 在把逐帧按键表交给 `emu.setInput` 之前，对
LEFT/RIGHT、UP/DOWN 两组对向键做了第二次消解；而这一层消解所依赖的 `btnOrder`
只有 nav/trigger 路径会填，`schedule` 路径（录像/inputlog 的 p1 字节）从不填。
后果是：**录像里 R+L 同按的帧被改写成「仅 L」、D+U 同按的帧被改写成「仅 U」**，
即 emu 腿收到的输入与录像不一致。这是 oracle 侧缺陷，不是 Rust 生产语义缺口。

## 1. 机制（读源确认）

`tools/scenario_capture.lua` 的 `inputPolled` 回调里：

- `schedule` 位解码：「算术式取位」只做 `btn[b] = true`，**不写 `btnOrder`**；
- nav（`sc-loop`）与 trigger 路径：`btnSeq = btnSeq + 1; btnOrder[b] = btnSeq`；
- 随后无条件执行对向键消解：

```lua
if btn.left and btn.right then
  if (btnOrder.left or 0) < (btnOrder.right or 0) then btn.left = nil else btn.right = nil end
end
```

schedule 来源时 `btnOrder.left == nil`、`btnOrder.right == nil`，两个 `(nil or 0)`
都是 `0`，`0 < 0` 为假 ⇒ 恒走 `else` ⇒ **`btn.right = nil`**。UP/DOWN 同理恒删 DOWN。

## 2. 影响面（A 级实测）

`movies/eien86-6725M.inputlog` 共 28 408 个**数据行**，其中 **R+L 同帧 31 帧**、
**D+U 同帧 15 帧**。这些帧在 emu 腿上被降为单向键。

> 【2026-11-19 更正（独立复核 `9bcd3c76` C 项）】帧号口径原先整体 **+8**——把 inputlog 的
> 9 行注释也算进了下标。按「数据行 1-based」口径的真值：**首个 R+L = f662（`0x03`）**，
> 序列 f662/664/666/668=`0x03`、**f670/f672=`0x43`**、f671=`0x42`、f663=`0x02`；
> `0x49` 在 **f1292**（f1293=`0x09`）；**首个 D+U = f17906**。口径可由
> `analysis/_gen/case-tmp-phase-1294.lua` 的 `schedule` 表直接核对
> （`[662]=3, [663]=2, [670]=67, [1292]=73, [1293]=9`）。影响面结论不变：
> 600 帧窗口 < f662，故当时的矩阵用例未受影响。

诱发链（取证子代理 `4f221313` 逐帧写序 + 反事实实验）：

1. f670 帧（`0x43`=R+L+B）与 f672 帧的 R+L 同按被改成「仅 L」⇒ `SamusDir` 反向、`SetSamusRunAccel`
   （`07:CCB7`）查 `RunAccelerationTbl`（`07:CCC0` = `$30,$D0`）得 `accX=0xD0`
   而非 `0x30`；
2. `$0311 SamusSubPixelX` 的累加相位随之漂移（窗口入口 rust `0x70` / emu `0x50`，差 `0x20`）；
3. 该段 Samus 在 `x=$030E=0x4C` 处向右受阻，只有当亚像素跨界使 `Temp00_Delta != 0`
   时才尝试走 1px（`HorzAccelerate ; 07:E35D` 段、`SamusMoveHorizontally ; 07:E31A`）；
4. 相位差 `0x20` 让「跨界→撞墙」落在晚一帧 ⇒ `CheckStopHorzMvmt ; 07:E365` →
   `SetSamusStand` 的 `[0,4]` 写组整体后移一帧 ⇒ `$0300` 的 Run/PntUp 在 f1293/f1294 互换。

反事实（决定性）：把 `.sched` 里 6 个 R+L 帧 `v &= ~0x01` 后重跑 rust，`$0300`
写序差异从 **43 帧降到 7 帧**（只剩 boot 早期 `{1,19,20,165,265,355,425}`，与本现象无关），
窗口 1292/1293/1294 与 emu 完全一致。

## 3. 真机语义（为何不能预先消解）

ROM 自己处理两键同按：`SamusRun ; 07:CCC2` → `samL11 ; 07:CD53`（`:2948-2953`）
`lda Joy1Status / and #RIGHT|LEFT / bne` → `samL12 ; 07:CD5F`（`:2955-2960`）
`jsr BitScan ; 07:E1E1`（`:6980-7001`，`lsr/bcs` **最低位优先**）。
`0x03` ⇒ bit0=RIGHT ⇒ `SamusDir=0` ⇒ 下帧 `accX=+0x30`；`0x02` ⇒ 左 ⇒ `accX=0xD0`。

因此注入器把两键原样交给 ROM 才是保真行为；DSL/nav 的「后写覆盖」只是脚本确定性便利，
不应作用于录像原始字节。

## 4. 修复

`tools/scenario_capture.lua`：给两组消解各加「两侧都来自有序源」前置条件
（`and btnOrder.left and btnOrder.right` / `and btnOrder.up and btnOrder.down`）。
schedule 原始位因此不再被工具改写，冲突交由 ROM 的 BitScan 决定。

## 5. 未定谳 / 待独立验证

- f1294 的 **215 px 不是这次 status 互换造成的**：注入器修复后 `$0300` 在 f1293/f1294
  已与 emu 相同（`04`/`01`），215 px 仍在。**首个未解释的差异在 f1293 `$0303 animFrame`**
  （emu `0x46` / rust `0x22`，同帧 `animResetIndex/animIndex/animDelay` 全同），
  需另起一轮（`SetSamusAnim ; 07:CF6B` 系列与 `$0303` 写点）——「animFrame 是根因」
  本身**尚未证明**，目前只能说到「首个未解释差异在这里」。
- f1294..f1298 的其余像素残差（33/163/184/167/1302=128）：9 个 Samus 字段已全同，
  OAM 影子区仍有差（f1294 5 B、f1295 60 B，集中在 slot 0..3）；**仅为假设（B/C 级）**：
  f1293 输入 `0x49` 含 B 键 ⇒ `FireSamusProjectile ; 07:D1EE` 的投射物精灵差异，未做单点隔离。
- **覆盖率（本段已补，复核 `9bcd3c76` E 项）**：修复当时 149 格里没有任何一格跨过
  冲突帧（唯一 `inputlog` 用例 `case-mechanism-door-transition.txt` 窗口 600 < 662），
  这是一条「改了但没被差分验证」的改动。现已入库永久用例
  `cases/case-mechanism-input-conflict.txt`（`frames 700` + 同一 inputlog，
  采点 661/662/670/671/672/673/674/676/678/680/700），它真正穿过 f662/f670/f672。
  该用例首次运行即暴露一个新分歧（f673 起，见 §6），说明这条覆盖是有效的。
- **混合来源语义（复核 A 项）**：运行时 nav/trigger 与录像原始位仍可在同一帧对向相撞。
  本工具的裁定是**不消解、交给 ROM 的 BitScan（`07:E1E1`，最低位优先）**，因此 RIGHT/DOWN
  恒胜；若将来需要「nav 覆盖录像位」，必须改成显式来源优先级，不得依赖 `btnOrder` 残留。
  `btnOrder` 已改为**每帧重置**（`tools/scenario_capture.lua`），否则跨帧陈旧序会让守卫在
  纯录像帧上误真、按旧序删掉原始位。
- 复现命令：`python3 tools/l4_diff_run.py run case-mechanism-input-conflict`（需
  `movies/eien86-6725M.inputlog`，不入库）。

## 6. 新分歧（本用例首次运行发现，未定谳）

f661..f672 全 0 px；**f673 起失配（326 px）**。两腿同帧 RAM 对比（2272 B 全量）：

| f | status `$0300` | dir `$030B` | speed_i `$0309` | subpix `$0311` | accelX `$030F` | x `$030E` |
|---|---|---|---|---|---|---|
| 671 | 01=01 | 01=01 | FE=FE | 30=30 | 14=14 | 72=72 |
| 672 | 01=01 | 01=01 | FE=FE | B0=B0 | 11=11 | 70=70 |
| 673 | 01=01 | 01=01 | FE=FE | 60=60 | 0E=0E | **70≠6F** |

速度/亚像素/方向/加速度**逐字节相同**，只有整数 `x` 少 1 ⇒ 不是积分算术，也不是输入面，
指向逐像素环里的挡判或滚动补偿（`MoveSamusLeft ; 07:E626` / `ObjectCheckMoveLeft ; 07:E7F0`
/ `ScrollLeft` 失败补偿）。1 px 之差随后把 `$005A` 的载入链推错一帧（f674：emu `08` /
rust `F0`），像素差级联到 326 px。定位进行中。


## 【2026-11-19 复核更正】Evidence 指针与 ppu.hex 同源盲区（r9 review-display-set-r2）

复核（pass-with-findings）抓出两处记账更正：

1. **PNG 证据指针错**：269d079c message 写「emu f673/f674/f676 三帧 PNG 逐字节相同」取自
   `shots_rust/…png`——实际成立的是 **emu 腿 `shots/case-mechanism-input-conflict_f0067*.png`**
   （f673/f674/f676 sha 全等 `472b8dd8ce846cb7`、f672↔f673 逐像素复算 13526px）；
   `shots_rust/` 里 f676 与 f673 **不**相同（`d8cf2ac4` vs `2e4e5872`）。本条更正后证据成立、
   指针归位。
2. **ppu.hex 窗内同源盲区**：269d079c 之后，载房窗内（phase==1）ppu.hex 快照派生自缝快照
   （实测 f673==f674 的 ppu.hex 全等）⇒ 窗内「四区全等」对 NT/palette 失去独立 oracle 性质。
   观测量收敛为：ram.hex（活体，独立）+ 窗尾 phase==2 帧的 ppu.hex。本案采集点已含
   676/678/680/700 四个窗尾/后续点，盲区被覆盖；后续新用例须保证窗尾有采集点。


## 【2026-11-19 复核更正 2】全链复核（review-fullchain-r1，pass-with-findings）四条收口

1. **P3-1 673 残差定性**（防下轮重复取证）：`capture(673)=tick(672)` 走的是**第一臂
   fresh 缝**（OAM x=121），而 emu f673 已显示 x=127 ⇒ 673 的 84px 是与 674 同根
   （NMI 停 ⇒ OAM DMA 停）但**不同臂**的残差——674 走第二臂（phase==1 复用缝）已修，
   673 需「窗首拍也复用缝」才能归零，属下一卡。
2. **P2-1 变体证据登记**：A/C 变体否决数字（elev f1207=13115px、scrollseam 1100
   266→290）只活在 269d079c message，现仓库不可复算，**置信度按 C 级**（elev 数字与
   elevator-evidence.md:86 历史值吻合为旁证）。B 变体影响面（唯一 674 变化）已被复核
   独立复算兜住。
3. **P3-2 已修**：classify_flaky 输出键并入 code_state（四元组），同格双码态不再互相
   覆盖；build_matrix 查找按前三元组匹配（键形态变更已同步）。变异验证：退回三元组键
   ⇒ 3 failed / 5 passed；恢复 8 passed。
4. **P3-3 未验证分支登记**：`seam_snap==None && room_load_phase==1`（MSS restore 直接入
   窗且首采集点先于首缝）回落活体渲染的路径无测试无产物（当前 20 案不触发），列为
   已知未验证分支，待补单测。
