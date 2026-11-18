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

`movies/eien86-6725M.inputlog` 共 28 408 行，其中 **R+L 同帧 31 帧**（首个 f670=`0x03`，
其后 f672/f674/f676 同为 `0x03`，f678/f680 为 `0x43`）、**D+U 同帧 15 帧**。
这些帧在 emu 腿上被降为单向键。

诱发链（取证子代理 `4f221313` 逐帧写序 + 反事实实验）：

1. f663 帧 R+L 同按被改成「仅 L」⇒ `SamusDir` 反向、`SetSamusRunAccel`
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
  已与 emu 相同（`04`/`01`），215 px 仍在。真因是 **f1293 `$0303 animFrame`**
  （emu `0x46` / rust `0x22`，同帧 `animResetIndex/animIndex/animDelay` 全同），
  需另起一轮（`SetSamusAnim ; 07:CF6B` 系列与 `$0303` 写点）。
- f1294..f1298 的其余像素残差（33/163/184/167/1302=128）：9 个 Samus 字段已全同，
  OAM 影子区仍有差（f1294 5 B、f1295 60 B，集中在 slot 0..3）；**仅为假设（B/C 级）**：
  f1293 输入 `0x49` 含 B 键 ⇒ `FireSamusProjectile ; 07:D1EE` 的投射物精灵差异，未做单点隔离。
- 本修复落地后必须重跑 L4 矩阵（唯一使用 `inputlog` 的用例是
  `cases/case-mechanism-door-transition.txt`）并更新 ratchet 基线；修复前不得视为完成。
