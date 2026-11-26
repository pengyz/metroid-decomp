# 可达门扫描：地面同层、无需跳跃、无需开门即可走入的门（#108）

**结论先行**

1. ROM 房间数据里**确实存在**「门站立面 = 房间主地面」的门：全三区共扫出 88 个门格，其中 **30 个门格在 17 个房间里站在房间主地面上**（下表）。
2. 但从**通电开局可走到的范围**里，**一个这样的门都没有**：从起点 `Room09`（MapPos 3/14）向左走停在 MapPos 2/14、向右走停在 5/14，两处都被门台/墙体挡住；`Room13`（5/14）右侧门的门台面在 y=128（tile 行 16），本房间主地面在 y=192（tile 行 24），从下方起跳被台底固实瓦截在 y=159（抬升 34px）——**够不到门台**。
3. 因此「开局→走进门→换房」这条冒烟**当前没有可落地对象**；能走进的门在 `Room1E`（Brinstar，MapPos 19/14 与 20/14，位于起点同一行的 16 格之外），需要先打通 `Room13` 的门台或支持「从指定房间开局」才能用纯键盘验证。这是 #104/#106 的真实前提，不是脚本问题。

## 1. 数据来源与解码口径（A 级）

| 数据 | 位置 | 格式 |
|---|---|---|
| metatile 定义 | `refs/m1disasm/SRC/<area>/metatiles.asm` 的 `MetatileDefs_{AREA}`（Brinstar `01:AEF0`） | 每 metatile 4 字节，顺序 **lower right, lower left, upper right, upper left**；索引 6 = `$A0`×4（竖滚门）、索引 7 = `$A1`×4（横滚门） |
| 结构定义 | `refs/m1disasm/SRC/<area>/structures.asm` | 每行 = 首字节该行 metatile 数 + 该数量的 metatile 索引；`$FF` 结束。`Structure03 = [01,06]×3`（3 metatile 高的竖滚门柱）、`Structure04 = [01,07]×3`（横滚门柱） |
| 房间结构摆放 | `refs/m1disasm/SRC/<area>/rooms.asm` | 首字节结构数，随后每结构 `($row<<4)+$col, _id_StructureXX, area`；`$FD` 结束结构表，其后是对象字节；`$FF` 结束 |
| 世界地图 | `refs/m1disasm/SRC/prg0_title.asm` 的 `WorldMap:`→`CopyMap:` 共 1024 字节（32×32） | `MapPos` 索引：`RoomNumber = WorldMapRAM[$7000 + MapPosY*32 + MapPosX]`（`07:E733` `GetRoomNum`，A 级） |

换算：房间 = 16×16 metatile 网格 → 32×32 tile 网格（metatile `(r,c)` 占 tile 行 `2r,2r+1`、列 `2c,2c+1`）。门柱 3 metatile 高 ⇒ 6 tile 行；门瓦块最下一行 +1 即「门站立面」所在 tile 行，站立高度 = `(行号)×8` px。

判据（与引擎一致，见 `crates/game-core/src/samus/physics.rs:1362` `collision_point_samus` 与 `IsWalkableTile 07:E800`）：
- tile `< $80`：固实；`$80-$9F`：可破坏；`$A0-$DF`：可穿过（含门瓦）；`$E0+`：特殊。
- 「地面同层」判定 = 门站立面 tile 行 == 该房间主地面行。主地面行取「从下往上第一个满足『本行 ≥16 连续固实、其上 ≥16 连续非固实』的行」（即 Samus 在房间出生落点站立的那一层）。

## 2. 结果：地面同层门清单（30 个门格 / 17 个房间）

门类型：`V` = `$A0`（竖滚门，左右方向进）、`H` = `$A1`（横滚门，上下方向进）。`col` 是 tile 列（门占 2 列，取左列）。所有门站立面均为 tile 行 16（y=128），这是本作门的标准高度。

| 区 | 房间 | 门 | 站立面 |
|---|---|---|---|
| Brinstar | `Room00` | V col=0、V col=30 | 行16 / y=128 |
| Brinstar | `Room0A` | H col=30 | 行16 / y=128 |
| Brinstar | `Room10` | V col=0、V col=30 | 行16 / y=128 |
| Brinstar | `Room1E` | V col=0、V col=30 | 行16 / y=128 |
| Norfair | `Room00` | H col=0、H col=30 | 行16 / y=128 |
| Norfair | `Room03` | V col=0、V col=30 | 行16 / y=128 |
| Norfair | `Room0B` | V col=0 | 行16 / y=128 |
| Norfair | `Room0D` | V col=0、V col=30 | 行16 / y=128 |
| Kraid | `Room00` | H col=0、H col=30 | 行16 / y=128 |
| Kraid | `Room06` | V col=0、V col=30 | 行16 / y=128 |
| Kraid | `Room15` | V col=0、V col=30 | 行16 / y=128 |
| Kraid | `Room1A` | V col=0、V col=30 | 行16 / y=128 |
| Kraid | `Room1B` | V col=0、V col=30 | 行16 / y=128 |
| Kraid | `Room21` | V col=0、V col=30 | 行16 / y=128 |
| Kraid | `Room22` | V col=0、V col=30 | 行16 / y=128 |
| Kraid | `Room23` | V col=30 | 行16 / y=128 |
| Kraid | `Room24` | V col=0 | 行16 / y=128 |

同时这 17 个房间里左右两门站立面相同、门间走廊（站立面上两行）无固实瓦 ⇒ 从左侧门走进、穿过房间、从右侧门走出在ROM 语义上没有物理障碍。

反例（只登记，不改）：`Room05` 左右门都在台面 y=128，但房间主地面在 y=192：同一房间里门的站立面与主地面不同层，从地面平台跳上去被台底瓦截断——就是 #104 已证的那间。

## 3. 为什么「开局可达」范围内一个都没有（A 级实测）

复跑命令（在 `lane/roll` 构建上，无 `--poke`）：

```bash
cd /home/peng/workspace/wt-roll
CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target cargo run -q -p game-shell-pc --bin game-shell-pc -- \
  --headless --script <脚本> --frames N --state-out /tmp/x.csv --shot-frames a,b --shot-dir /tmp/shots
```

- `9-9 START` / `18-18 START` / `200-8000 RIGHT`：房间序列 `(255,3,14) → (20,4,14) → (19,5,14)` 后**停住**（MapPos 5/14，Samus x=0xDC，scroll_x=0），8000 帧内 `$56/$58` 恒 0。
- `200-1450 RIGHT` + `1451-1451 A+RIGHT` + `1452-1500 A+RIGHT` + `1501-1600 RIGHT`：Samus 起跳峰值 y=0x9F（从 y=0xC1 抬升 34px），随后落回 0xC1；门台面在 y=128，差 31px 被台底固实瓦挡住；`probe_tile` 在起跳过程中为 `02/00/04`（可穿瓦），说明**不是撞墙失败，是天花板把跳跃截断**。
- `200-3000 LEFT`：只到 MapPos 2/14，`door_slots` 全程 `0/0/0/0`、`door_tiles` 0/0。
- 世界地图核对（A 级）：`WorldMap[(3<<5)+14] = $09`、`[(4<<5)+14] = $14`、`[(5<<5)+14] = $13`，与引擎读出的房间号 9/20/19 完全一致，证明「MapPos 行 14 = Brinstar 走廊」的解读正确；`Room1E`（0x1E）出现在 `(19,14)`、`(20,14)` 两格（同一行的 16 格外）。`Room00/0A/10` 的同号房间在不同行（16 行以外），属别的区或同号不同房间，不能算「Brinstar 可达」。

## 4. 对 #104 / #106 的交接

- **#104** 若要继续「可玩穿门」验证，只有两条路：① 支持一个**仅启动用**的房号参数（或 `.mss` 存档）把 Samus 放进 `Room1E`/`Room00` 这类地面同层门房间；② 先修 `Room13` 台底/跳跃可达性（属物理/房间数据面，需另开卡）。
- **#106** 的冒烟脚本（`tools/playable_door_smoke.sh`）当前红灯 `NO_DOOR_TRIGGER` 的根因已更新为：**起点可达区域内不存在门格可触达的门**，而不是脚本按键问题（按键边沿、并集窗口都已验证正确）。
- 本卡与 #104 的实测一致：`Room05`（MapPos 5/14 = 房间号 19 = `Room13`）门台高于主地面，台底固实瓦把跳跃截在 34px。

## 5. 复跑扫描（analysis 内脚本，不入库）

脚本：`analysis/reachable_doors/scan.py`（只读 `refs/m1disasm`）。核心算法：
1. 解析 `MetatileDefs_{AREA}` 为 4 字节组表；
2. 解析 `structures.asm` 每行 `计数 + metatile 索引` 序列（`$FF` 结束）；
3. 解析 `rooms.asm` 的 `(row<<4|col, struct id, area)` 摆放表；
4. 摊成 32×32 tile 网格 → 找 `$A0/$A1` 瓦块 → 取块最下一行 +1 为站立面；
5. 主地面行 = 从下往上第一个「本行 ≥16 连续固实、其上一行 ≥16 连续非固实」；
6. 站立面 == 主地面 ⇒ 地面同层。

复跑：

```bash
cd /home/peng/workspace/metroid-decomp && python3 analysis/reachable_doors/scan.py
```

输出 88 行门表 + 「地面同层」30 行小节；本文件的表由该输出直接生成。
