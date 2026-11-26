# 门入场不切房：`samus_enter_door` 丢方向码（#102）

结论：**可玩构建里「走进门、门动画照走、房间不换」的根因是
`SamusInDoor` 的方向码丢失**——ROM 写 `DoorEntryStatus = X|$80`（X 是 1..4 的门方向码），
Rust 写成了 `$56|$80`，而进入前置保证 `$56==0` ⇒ 恒写 `$80`。
`ScrollDoor` 对 `$56` 做四次 `dex/bne`，`$80` 走完全部 `bne` 后直接 `Exit15`，
一个像素都不滚；随后 `SamusDoor` 的 `asl` 把 `$80` 清成 `0`（而不是 `1`），
门链在「进门 18 帧」后无声结束，房间号/MapPos 不变。

## A 级证据

### 1. ROM 原文（`refs/m1disasm/SRC/areas_common.asm`）

```
SetDoorEntryInfo: ; $8B53
    txa
    sta DoorScrollStatus      ; $57 ← X（方向码 1..4）
    jsr SamusInDoor
...
SamusInDoor: ; $8B74
    ora #$80                  ; A 此刻是 SetDoorEntryInfo 里的 txa 结果 = X
    sta DoorEntryStatus
```

`X` 的取值就在紧邻上方：横滚 `ldx #$02`、左门 `dex`（右门不改）；竖滚 `ldx #$04`、
按 `ScrollY`/`PPUCTRL^Samus.hi` 决定是否 `dex`。⇒ `$56 = X | $80`。

`ScrollDoor ; 07:E1F1` 的判据是 `ldx $56 / beq exit / dex / bne ...` 逐次比对 1/2/3/4，
`$80` 与四个值都不等，四条 `bne` 全落 `Exit15`——**门永远不会开始滚屏**。

`SamusDoor ; 07:D3A8` 在 `DoorDelay` 归零后做 `asl; bcc; lsr; sta $56`：
`$81 → $01`（继续走滚屏），`$80 → $00`（直接进清场分支）。这一条是「丢方向码」的
直接后果分叉。

### 2. Rust 落点

`crates/game-core/src/samus/physics.rs:1437-1439`（#102 取证时的行号）：

```rust
// SamusInDoor（$8B74）：$56|=$80
let de = mem.read_ram(crate::ram_map::DOOR_ENTRY_STATUS);
mem.write_ram(crate::ram_map::DOOR_ENTRY_STATUS, de | 0x80);
```

上一处已经算出了方向码 `x`（`mem.write_ram(0x0057, x)`），这里没有用它。

### 3. 运行实验（可复跑：`tools/door_entry_repro.sh`）

`game-shell-pc --headless --script /tmp/s102/poke.txt --poke 1500:0x58=2`
（脚本先按 TAS 开局，再一路 `RIGHT` 到 map5 的右门墙边；`--poke` 把
`SamusDoorData($58)` 摆成 `2`，即「Samus 已踩在门格瓦上」，从而把门链与
「能不能走到门格」解耦——`$58` 的写入面在 `collision_point_samus`，不受本改动影响）。

- **改前**（HEAD）：f1500 `$56=$80`、`$57=1`、`$58=$21`、`DoorDelay=18`；
  f1518 `$56=0`、`DoorDelay=0`；房间号 `255`、MapPos `5/14`、ScrollX `0` **全程不变**。
- **改后**（把 `de | 0x80` 换成 `x | 0x80`，一行）：f1500 `$56=$81`；
  f1519 `$56=1`、房间号 `255→24`、MapPos `5/14→6/14`；f1523 起 `ScrollX` 2→6→10… 逐帧推进。

同一份输入、同一份资产包，只有这一行不同 ⇒ 门链的其余件（`ScrollDoor`、
`SamusDoor`、`check_update_name_table`、`setup_room`）都能把房间切过去。

固化命令（`tools/door_entry_repro.sh [帧数]`，默认 2400）：

- HEAD `c240e1b7`：`注入前 room=255 map=5/14；注入后首个门事件 $56=128` /
  `注入后 40 帧内：房间号/MapPos 均未变化` ⇒ 退出码 1（`FAIL`）。
- 打上本文件的修复补丁：`$56=129` / `注入后 40 帧内：room=24 map=6/14`
  ⇒ 退出码 0（`PASS`）。

## 为什么之前的证据都没抓到

- L4 `case-002-door-blue-interaction` 的 8 个采集帧，emu/rust 两侧 `$56/$57/$58` 全 0，
  截图只有走廊：该用例从未走到门，门 6 格通过不构成门链证据。
- 100% TAS 重放（`analysis/tas-100`，120347 帧）`$56/$58` 恒 0，房间号在 f22454 起反复
  回 `(0,0,0)`：这卷 fm2 绑定 FCEUX，在 Mesen 里重放到 Brinstar 段就死亡重开，
  也没有门事件。「TAS 覆盖到 30000 帧」应更正为「重放到 f~22k 即死循环，全程无门事件」。
- 可玩构建里从通电开局用走路+跳跃能到达的位置**没有可站上去的门格**（唯一找到的
  map5 门在中高台，跳跃峰值比平台低），所以纯键盘脚本也复现不出「走进门」这一步；
  本 findings 用 `--poke $58` 越过这一步，直接验证门链本身。

## 验收标准（建议按此三条，#102 原卡第 1 条按字面不可达）

1. `tools/door_entry_repro.sh` 退出码 0：`$56` 带方向码（`0x81..0x84`）且注入后 40 帧内
   RoomNumber 或 MapPos 变化；附注入前/后两帧截图。
2. 回归不回退：`cargo test --workspace` ≥1229 全绿、`gates/cli.py check`
   `PASS=24 FAIL=0 SKIP=3 TOLERATED=1`、`l4_diff_run.py run-all` **171/172**
   （唯一 diff 仍是 `case-mechanism-input-conflict f674=84px`）。
3. 本 findings 的「为什么之前的证据都没抓到」保留：L4 的 door 6 格与 100% TAS
   都**不能**作为门链证据。

## 待办（跨出 #102 写面）

修复位置在 `crates/game-core/src/samus/physics.rs`，#102 的写面是
`crates/game-core/src/world/` + `crates/game-shell/game-shell-pc/`，
按 PMO 的边界规则不在本卡内改。建议同卡追加写面或拆卡，改动与验证：

1. `samus_enter_door` 里写 `x | 0x80`（`x` 已在上面算出），删掉重新读 `$56` 的两行；
2. 在 `physics.rs` 的门测试里补正反例：`$58=2`、`ScrollDir` 横滚、`Samus.x` 高位为 1
   ⇒ `$56==0x81`；`Samus.x` 高位为 0 ⇒ `$56==0x82`；
3. 回归：`python3 gates/cli.py check`、`cargo test --workspace`、`tools/l4_diff_run.py run-all`
   （门 6 格若因此变化，按 §「为什么之前的证据都没抓到」先确认是变好还是变坏）。

预演数字（在 lane 工作树上临时打过补丁、测完已还原）：`cargo test --workspace`
**1229 passed 0 failed**（新增 1 条方向码正反例）；`l4_diff_run.py run-all --skip-emu`
**21 案 / 172 格 match 171 diff 1**（与基线同，唯一 diff 仍是
`case-mechanism-input-conflict f674=84px`）。
