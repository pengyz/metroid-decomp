# 可玩端到端冒烟：走进门 → 换房（#106）

这条冒烟回答一个对拍答不出来的问题：**玩家用键盘能不能自己走进门、把房间换掉。**
它不注入任何内存（不用 `--poke`），只喂按键脚本，判据全落在逐帧状态 CSV 与换房后那一帧的截图上。

## 一条命令

```bash
# 在 wt-roll 或主线检出里（需要 assets/metroid.pack 存在）
CARGO_TARGET_DIR=target tools/playable_door_smoke.sh          # 阶段 1：横滚门（当前 PASS）
DOOR_SHOTS=/tmp/smoke tools/playable_door_smoke.sh            # 截图落指定目录
tools/playable_door_smoke.sh --self-test                      # 判定器四态自检（不跑游戏）
tools/playable_door_smoke.sh --check-invalid-room             # 负对照：非法房号必须被拒
tools/playable_door_smoke.sh --stage 2                        # 阶段 2：竖向滚屏门（当前 PASS）
# 换输入脚本而不改本文件：
tools/playable_door_smoke.sh --script /tmp/door_input.txt --frames 1800 --shot-frames 1470
```

`--script` 读不到时退出码 2 并打印路径；日志里会注明「脚本来自 xxx」，避免误以为跑的是内置脚本。

退出码就是结论（0 才算通过），CI/回归直接看码：

| 退出码 | 判定 | 含义 |
|---|---|---|
| 0 | `PASS` | 触发门 → 换房 → 新房间可渲染 |
| 3 | `NO_DOOR_TRIGGER` | `$56` 全程 0、`$58` 峰值 0 ⇒ Samus 从未踩到门格（可进入性/门瓦问题） |
| 4 | `NO_SWITCH` | 门事件已触发，但 90 帧内 RoomNumber/MapPos 未变（滚屏或 `SetupRoom` 断链） |
| 5 | `BLACK_SCREEN` | 换房后首帧主体区非黑像素 < 5%（黑屏/未画房） |
| 6 | `WRONG_DIRECTION` | 门触发了、也换房了，但方向码不在该阶段期望集合里（阶段1=1,2；阶段2=3,4） |
| 1 | 运行失败 | 二进制/参数/产物读取失败 |

## 阶段 1：Brinstar 房 5 右门（当前 PASS，实测读数）

脚本（内置，`--start-pos` 只把 Samus 摆到门台上，其余全键盘）：

```
9-9 START
18-18 START
200-1700 RIGHT
925-945 B          # 开锁蓝门：Closed($4E) → Open($FF) 才能走进去
```

```
--start-pos 0xD0,0x6F --start-frame 900
```

实跑输出（A 级，`CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target`，约 15s）：

```
[冒烟] 阶段 1：Brinstar 房 5 右门（横滚门，$56=$81）
[触发] f=948 $56=129 $58=17 旧房间=255 MapPos=5/14
[方向码序列] $56: 129×18 → 1×130 → 5×31
[换房] f=967 新房间=24 MapPos=6/14
[渲染] …/f01200.ppm 主体区非黑占比=0.2605 颜色数=13
[PASS] 走进门 → 触发门 → 换房 → 新房间可渲染
```

逐项对应卡面验收：`$58≠0`（f948=0x11）→ `$56=129`（=$81，方向码 1=右横滚）→ `$56=1` 滚屏相位
（f966）→ f967 房间号 255→24、MapPos 5/14→6/14 → f968 起 `ScrollX=2,4,6…` → 新房间首帧可渲染。

## 阶段 1 为什么需要 `--start-pos`

真实路径「从通电开局走到这扇门」走不到：房 5 的门在屏幕 y=80..127 的中高台，门台顶约 y=128，
Samus 站立脚底约 y=193；在本间起跳会被头顶实体瓦截断（`200-1400 RIGHT` + `1401-1470 A+RIGHT`：
`samus_xy` 由 `0DC/0C1` 升到 `0DC/09F`，即离地 0x22 = 34px 就回落，`probe_tile` 顶点读到 `4D`），
横向也推不动。可进入性本身归 #104/#108 另论（#108 已证起点可达区没有任何地面同层门）。

因此本卡验收由主理人改写为「`--start-*` 只做启动定位，不改游戏逻辑」：参数在指定帧写
`RoomNumber`/`MapPos`/`Samus` 坐标，载房仍走引擎自己的判据 `room_load::is_new_load_request`
（`crates/game-core/src/world/room_load.rs:228`），坐标要等 `room_load_phase==0` 后才落，否则会被
装载覆写（实测 f20 写 `028/06F` 被改回 `080/0B0`）。房号非法或 200 帧内未被装载 ⇒ 非 0 退出并打印
「房间号不被接受」。

```bash
tools/playable_door_smoke.sh --check-invalid-room
# [CHECK:PASS] --start-room 0xF5 被拒绝（rc=2，stderr 有「房间号不被接受」）
```

## 阶段 2：Room24 左门回程（竖向滚屏，当前 PASS）

卡面要求「横滚门与另一方向门各一道」。竖向滚屏门在**同一对房间的回程**上就能拿到：
房 5 右门（`$81` 右横滚）走进去到 Room24（MapPos 6/14），再从这个门的另一侧往回走，
引擎按 `ScrollDir` 判成竖向上滚，方向码 `$84`。

脚本（内置）：

```
9-9 START
18-18 START
200-1700 RIGHT
925-945 B          # 开锁 → 进房 5 右门 → Room24
1200-2400 LEFT     # 走向 Room24 左门（走回来时的那扇）
2450-2470 B        # 开锁
2480-2900 LEFT     # 走进去 → 竖滚回房 5
```

```
--start-pos 0xD0,0x6F --start-frame 900
```

实测读数（A 级，无 `--poke`）：

```
[触发] f=2490 $56=132 $58=17 旧房间=255 MapPos=6/14
[方向码序列] $56: 132×18 → 4×1 → 2×130 → 5×31
[换房] f=2509 新房间=19 MapPos=5/14 方向码=4
[渲染] f02850 主体区非黑占比=0.0788 颜色数=11
[PASS] rc=0
```

`$56=132` = `$84` = 方向码 4（竖向上滚）| `$80` 旗；与阶段 1 的 `$81` 不同向。

### 启动定位 `--start-area`（本卡新增）

`--start-area 0..4`（0=Brinstar 1=Norfair 2=Kraid 3=Tourian 4=Ridley）在启动帧把
`InArea($0074)` 低 4 位改掉，再写 `RoomNumber`/`MapPos`；载房仍走引擎判据
`room_load::is_new_load_request`（`crates/game-core/src/world/room_load.rs:228`），不改游戏逻辑。

硬证据：同一 MapPos `(29,6)`、同一 `--start-room 0x06`，只改 `--start-area`：

| area | 结果（700 帧后） |
|---|---|
| 0（Brinstar） | `door_slots=0/0/0/0`、`door_tiles=0/0`（Brinstar Room06 没有门对象） |
| 2（Kraid） | `door_slots=2/0/0/2`、`door_tiles=24/0`（Kraid Room06 的 24 个 `$A0` 门瓦） |

两次的 WRAM dump md5 不同（`d7dec42b…` vs `29dff04c…`），说明换的是整张区表，不是只改了一个字节。

## 判定器自检（不跑游戏）

判定规则抽在 `tools/playable_door_judge.py`，脚本自带合成样本自检：

```bash
tools/playable_door_smoke.sh --self-test     # rc=0 ⇒ 四态判定与退出码都符合预期
```

```
[自检] 判定器四态（合成样本，不跑游戏）
  [ok]   PASS                   rc=0 [PASS] 走进门 → 触发门 → 换房 → 新房间可渲染
  [ok]   NO_DOOR_TRIGGER        rc=3 [FAIL:NO_DOOR_TRIGGER] …
  [ok]   NO_SWITCH              rc=4 [FAIL:NO_SWITCH] …
  [ok]   BLACK_SCREEN           rc=5 [FAIL:BLACK_SCREEN] …
  [ok]   WRONG_DIRECTION        rc=6 [FAIL:WRONG_DIRECTION] …
[SELFTEST:PASS] 四态判定与退出码均符合预期
```

## 判据背后的两个刻度（不是拍脑袋）

- **黑屏判据**：跳过顶部 32 行 HUD，在 y=32..232 的主体区统计非黑像素占比。
  实测正常帧 0.1549、0.1555、0.1558、0.1689（7–13 色），全黑帧 0.0000 ⇒ 阈值取 5%，离正常值约 3 倍余量。
  不用 #102 的「颜色数 ≥2」：只有 HUD 的画面也有十几色，那个尺子分不出「黑屏」。
- **换房窗口**：门事件后 90 帧。`$56` 从 `方向码|$80` 到 DoorDelay 归零、滚完屏、`SetupRoom` 落地实测 19 帧（f948→f967），90 帧留足 4 倍余量。

## 边界

- 只读：脚本只调用 `game-shell-pc` 的 `--headless/--script/--start-*/--state-out/--shot-*`，不写 `--poke`、不改游戏逻辑。
- `--start-*`（含 `--start-area`）只做启动定位（改写验收口径），不是「玩家从通电开局可走到门」的证明；这一点在报告里前置声明。
- 产物落 `mktemp -d`，不污染仓库；`DOOR_SHOTS` 可指定截图目录。
- 本冒烟只证明「可玩路径能穿门换房」，不替代 L4 像素对拍，也不构成「整机可玩性已验证」。
- 历史提醒：L4 的 door 三案（`case-002` 8 格、`case-mechanism-door-transition` 6 格、`case-mechanism-door-negative` 3 格）采集帧里 `$56/$58` 恒 0（空跑），它们「绿」不代表门链通过。
