# `$0317-$03FF` 语义规格（B2-a 取证）

> 目标：给 `nesmap-dump` 里没有 note 的 233 B（`$0317-$03FF`，`0x3FF-0x317+1 = 0xE9 = 233`）做取证规格，回答「这段到底是什么、能不能 typed 化」。
> 结论先说（§0），逐字段与证据在 §2 以后。**卡面的「每区状态数组尾」假设在 §6 被否证**。
> 本文件只取证，不改 `crates/`；涉及实现卡的判断写在 §8 的「建议」列，由实现卡决定。

## 0. 结论摘要

| # | 结论 | 级 |
|---|---|---|
| 1 | `$0317-$03FF` **不是**独立数组，而是 **`$0300-$03FF` 16 槽 × 16 B 通用对象池（`Objects`）的尾段**：`$0310` 起是槽 1…15。 | A |
| 2 | 对象结构 `Object` 共 16 字段（`status`…`data0F`），**字段偏移只有 `0x00-$0F`**；池内 16 槽共享同一套偏移，所以「槽 N 的字段」= `$0300 + N*16 + 偏移`。 | A |
| 3 | 各槽有**语义别名**（`Samus`/`Elevator`/`Statue`/`Doors`/`SamusProjectiles`/`PowerUpDraw`），但 `Object0310`/`Object0330`/`Object0350`/`Object0370`/`Object03C0` 是**无语义的占位别名**，代码里只有定义、没有引用（`Object0310` 全文 1 处，见 §9 命令 5）。 | A（sym 表 + asm 引用计数） |
| 4 | `$0310-$0316` **不是** 槽 1 的对象字段，而是 Samus 的**扩展物理面**（`SamusSubPixelY/X`…`SamusHorzSpeedMax`）；因此 `Object0310` 这个别名是被覆盖的假槽——槽 1 在运行时不被当作对象使用。 | B（常量表 + 运行值型态 + Rust 同名常量；§3） |
| 5 | `$0320-$032F` 是**双用途**：游戏内是 `Elevator` 槽，标题画面（密码/开始选择）借用同一段做 `PasswordCursor/InputRow/InputColumn/StartContinue`。两者不共存（标题态没有游戏实体）。 | A（两侧写点都定位到具体地址；§4） |
| 6 | 槽 3（`$0330`）、槽 5（`$0350`）、槽 12（`$03C0`）在 11 116 份 trace 中**基本恒为 0**（非零帧个位数），可判为**预留槽**，只保留「被清零」语义。 | B（§5 的运行统计） |
| 7 | Rust 侧**已经把整段建模成对象池**（`world/object.rs` 的 `OBJECTS_BASE=0x0300/SLOT_STRIDE=16/SLOT_COUNT=16` + 字段偏移），并另在 `elevator.rs`/`statue.rs`/`room_draw.rs`/`samus/handler.rs` 里给具体槽具名；缺的是**逐槽语义名与预留槽的显式标注**，不是「整段缺模型」。 | A |
| 8 | `crates/game-core/src/sound/music.rs:122` 的 `music_schedule()` 读 `ram[0x0331]`，但 `$0331` 在反汇编里**零引用**，且该函数**零生产调用者**（只有它自己的测试）。属于「按地址发明语义」的残留，建议实现卡删掉或改成正确来源。 | A（引用计数 0）+ B（语义无据） |

## 1. 数据模型：16 槽对象池

符号表（`refs/m1disasm/out/M1_NES_NTSC.sym`，wla 导出，`bank:addr` 形式）：

```
00000300 Objects          00000300 Samus
00000310 Object0310       00000320 Elevator        00000330 Object0330
00000340 PowerUpDraw      00000350 Object0350        00000360 Statue
00000370 Object0370       00000380 Doors            000003c0 Object03C0
000003d0 SamusProjectiles
```

`Doors instanceof Object 4`、`SamusProjectiles instanceof Object 3` ⇒ 门占槽 8–11（`$0380/$0390/$03A0/$03B0`），子弹占槽 13–15（`$03D0/$03E0/$03F0`）。其余单实例别名各占一槽。

`Object` 字段表（`refs/m1disasm/SRC/constants_ram.asm:217-239`，行号按该文件）：

| 偏移 | 字段 | 地址（槽 0） | 语义 |
|---|---|---|---|
| `0x00` | `status` | `$0300` | 0 = 槽未使用；非 0 = 在用（值域见 `constants.asm:135-141`） |
| `0x01` | `radiusY` | `$0301` | 中心到上/下的像素距离 |
| `0x02` | `radiusX` | `$0302` | 中心到左/右的像素距离 |
| `0x03` | `animFrame` | `$0303` | ×2 = 帧指针表下标 |
| `0x04` | `animDelay` | `$0304` | 帧间等待帧数 |
| `0x05` | `animResetIndex` | `$0305` | 播完回绕到的下标 − 1 |
| `0x06` | `animIndex` | `$0306` | `ObjectAnimIndexTbl` 当前下标 |
| `0x07` | `data07` | `$0307` | 多义：槽 0 = `SamusOnElevator`；门槽 = `DoorType`；雕像 = `RidleyStatueIsHit` |
| `0x08` | `speedY` | `$0308` | i8，MSB=向上/向下 |
| `0x09` | `speedX` | `$0309` | i8，MSB=向左/向右 |
| `0x0A` | `isHit` | `$030A` | 命中/接触位（Samus 特殊位义见 `constants_ram.asm:884-889`） |
| `0x0B` | `onScreen` | `$030B` | 1 = 在屏内 |
| `0x0C` | `hi` | `$030C` | 覆盖的名表（0 或 1） |
| `0x0D` | `y` | `$030D` | 房间坐标 y |
| `0x0E` | `x` | `$030E` | 房间坐标 x |
| `0x0F` | `data0F` | `$030F` | 多义：门 = `DoorHitPoints`；子弹 = `SamusProjectileDieDelay`；电梯 = `ElevatorType`；雕像 = `KraidStatueY` |

由此 `$0317-$03FF` 的字段级含义完全由「槽号 + 偏移」决定，**不需要**新的结构定义。例如 `$0317` = 槽 1 的 `data07`，`$039F` = 槽 9（门 2）的 `data0F`。

槽内清零（`prg7_engine.asm:1866`，`LC836` 循环）：

```
        cpx #$FF                        ;Check to see if more RAM to clear in $300 thru $3FE. (off-by-one bug)
        bcs LC83D
            sta Objects.0.status,x      ;Clear RAM $300 thru $3FE.
```

循环在 `x = $FF` 时退出 ⇒ 只清 `$0300-$03FE`，**`$03FF` 漏清**（原版 off-by-one，Stage A 需保真）。注意清的是每个槽的 `status`（步长 16，`inx` 后由外层加），不是整段 256 B。

## 2. 逐槽表（`$0310-$03FF`）

证据级：A = 常量/符号表/写点直证；B = 多处间接证据一致；C = 推断。

| 槽 | 地址 | 别名（sym） | 实际语义 | 写点（举例） | 读点（举例） | 级 |
|---|---|---|---|---|---|---|
| 1 | `$0310-$0316` | `Object0310`（假） | **Samus 扩展物理面**：`SamusSubPixelY/X`、`SamusSpeedSubPixelY/X`、`SamusAccelY/X`、`SamusHorzSpeedMax` | 物理更新（斜坡/加速度径） | 物理积分 | B |
| 1 | `$0317-$031F` | — | 未使用（11 116 帧里仅 21 次非零，见 §5） | 无 | 无 | A |
| 2 | `$0320-$032F` | `Elevator` | 游戏内 = 电梯槽；标题态 = 密码/开始选择字段 | `SpawnElevatorRoutine 07:EC09`；标题写 `$0320-$0325` | `UpdateElevator 07:D7B3`；标题读 | A |
| 3 | `$0330-$033F` | `Object0330`（占位） | 预留槽（恒 0） | 无 | 无 | B |
| 4 | `$0340-$034F` | `PowerUpDraw` | 掉落道具（与 Skree 弹道共用模板）的绘制槽 | `07:ECxx` 道具生成写 `y/x/hi/animFrame` | `ObjectDraw` | A |
| 5 | `$0350-$035F` | `Object0350`（占位） | 预留槽（恒 0） | 无 | 无 | B |
| 6 | `$0360-$036F` | `Statue` | 雕像槽（Kraid/Ridley 共用，靠 `data0F` 和 `data07` 区分） | `LoadStatues 07:EC2F`、`UpdateStatue_Raise 07:DA3D` | `07:D9D4`/`07:DADA` 链 | A |
| 7 | `$0370-$037F` | `Object0370` | 仅 `$0370` 有名字 `RidleyStatueY`（雕像绘制时复制目标 y） | `StartRaising 07:DA7C` 附近写 | 雕像绘制 | B |
| 8-11 | `$0380-$03BF` | `Doors` ×4 | 门槽 0-3；`data07`=`DoorType`、`data0F`=`DoorHitPoints` | `LoadDoor 07:EC00` 起逐槽填 | `ScrollDoor`/`SamusEnterDoor` | A |
| 12 | `$03C0-$03CF` | `Object03C0`（占位） | 预留槽（恒 0） | 无 | 无 | B |
| 13-15 | `$03D0-$03FF` | `SamusProjectiles` ×3 | Samus 子弹/导弹槽；`data0F`=`SamusProjectileDieDelay` | 发弹 `SpawnSamusProjectile` | `UpdateSamusProjectiles` | A |

「被覆盖」的两处要说清：

- 槽 1 头 7 B（`$0310-$0316`）被 Samus 扩展面覆盖，所以 `Object0310` 这个 sym 别名**从未被引用**（asm 引用计数 0，见 §5 命令）。这不是笔误：`constants_ram.asm:882` 明写「Samus RAM ($0300-$0316)」。
- 槽 2 的 `Elevator` 与标题态 `PasswordCursor` 等共用地址，靠**游戏模式**互斥，不是同一结构的多义字段（§4 给两侧写点）。

## 3. 为什么判定 `$0310-$0316` 是 Samus 扩展面而不是槽 1 字段（B 级）

三条独立证据，任何一条单独都只是提示，合起来才够 B：

1. **常量表**：`constants_ram.asm:893-901` 在 `.enum $0310` 里逐字节命名 `SamusSubPixelY…SamusHorzSpeedMax`，并把上一段标成「Samus RAM. ($0300-$0316)」。
2. **运行值型态**：游戏内 trace 里 `$0310-$0316` 每帧都在变、取值遍布 `$00-$FF`（如 `6c 80 00 80 00 00 18`），而对象槽若无 `status`（`$0310`）合法值（0-9）就不该有字段更新。若当作槽 1 的 `status`，取值如 `0x6C`、`0xBC` **不落在合法状态集**里。
3. **Rust 同名常量**：`crates/game-core/src/ram_map.rs:334-381` 把 `$0310-$0316` 命名为 `SAMUS_SUBPIXEL_Y/X`、`SAMUS_SPEED_SUBPIXEL_Y/X`、`SAMUS_ACCEL_Y/X`、`SAMUS_HORZ_SPEED_MAX`，与 ROM 注释一一对应。

## 4. `$0320-$032F` 双用途（A 级，两侧写点都定位到地址）

**游戏内：`Elevator` 槽**（`SpawnElevatorRoutine 07:EC09`，常量见 `constants_ram.asm:905-920`）：

| 地址 | 字段 | 写值（进入时） |
|---|---|---|
| `$0320` | `status` | 1 |
| `$0323` | `hi` | `$23` |
| `$0328` | `speedY` | `$00`（注释明说「写了但从不读」） |
| `$032C` | `animFrame` | 名表号 |
| `$032D` | `y` | `$83` |
| `$032E` | `x` | `$80` |
| `$032F` | `ElevatorType`（`data0F`） | bit7=上/下行，低半字节=线路号；`$8F` = 结局电梯 |

**标题态：密码/开始选择**（`constants_ram.asm:950-958` 的 `.enum $0320`，写点在 `prg0_title.asm`）：

| 地址 | 字段 | 语义 |
|---|---|---|
| `$0320` | `PasswordCursor` | 密码写入位置（`$00-$17`） |
| `$0321` | `InputRow` | 字符行（`$00-$04`） |
| `$0322` | `InputColumn` | 字符列（`$00-$0C`） |
| `$0323` | `SpareMem0323` | 无具名用途（与电梯态 `hi` 同址） |
| `$0324` | `PasswordStat00` | **注释原文「Does not appear to have a function」** ⇒ 只能标 `_todo_`，不许命名 |
| `$0325` | `StartContinue` | 0 = START 选中，1 = CONTINUE 选中 |

判据：两侧写点写入的**值域不同**（电梯 `$0320` 恒 1，密码态是游标 0-0x17），且运行时二者从不同帧出现（同一帧内不会既是菜单又是游戏房）。

## 5. 运行期统计：哪些槽真的在用（B 级，用 11 116 份 trace 的 RAM 页）

窗口 `analysis/traces/*.ram.hex`（每份前 2048 B 就是 `$0000-$07FF`），统计 `$0317-$03FF` 每字节非零帧数：

```python
import glob, pathlib, collections
nz = collections.Counter(); tot = 0
for p in sorted(glob.glob('analysis/traces/*.ram.hex')):
    b = bytes.fromhex(pathlib.Path(p).read_text().strip())
    tot += 1
    for a in range(0x317, 0x400):
        if b[a]: nz[a] += 1
print(tot)
for a in range(0x310, 0x400, 0x10):
    print(f'{a:03X}', sum(nz.get(a + i, 0) for i in range(16)))
```

实测输出（按 16 B 槽聚合，满帧数 11 116）：

| 槽 | 地址 | 非零字节数合计 | 判定 |
|---|---|---|---|
| 0 | `$0300` | 不在本统计窗口（起点 `$0317`），Samus 槽每帧在用 | 在用 |
| 1 | `$0310` | **21**（只统计了尾 9 B：`$0317-$031F`） | 尾 9 B 基本闲置；头部 `$0310-$0316` 属 Samus 扩展面、不在窗口内 |
| 2 | `$0320` | **55 287** | 密码/电梯双用途，全段最高 |
| 3 | `$0330` | **25** | 预留 |
| 4 | `$0340` | **7 033** | 道具绘制 |
| 5 | `$0350` | **25** | 预留 |
| 6 | `$0360` | **510** | 雕像（仅雕像房） |
| 7 | `$0370` | **83** | 雕像辅助 |
| 8 | `$0380` | **229** | 门 0 |
| 9 | `$0390` | **26 434** | 门 1 |
| 10 | `$03A0` | **30 583** | 门 2 |
| 11 | `$03B0` | **228** | 门 3 |
| 12 | `$03C0` | **2** | 预留 |
| 13 | `$03D0` | **9 554** | Samus 子弹 0 |
| 14 | `$03E0` | **8 164** | Samus 子弹 1 |
| 15 | `$03F0` | **3 553** | Samus 子弹 2 |

对照 Rust：`world/object.rs` 已经把整段当 16 槽池（`OBJECTS_BASE/SLOT_STRIDE/SLOT_COUNT`），并用 `obj::*` 偏移访问；但**没有**「槽 1 头 7 B 属于 Samus 扩展面」「槽 3/5/12 是预留」这类语义标注。

## 6. 「每区状态数组尾」假设：**否证**（A 级）

假设：`$0317-$03FF` 是按区域（area）分块的状态数组尾，每区一段。

三条反证：

1. **寻址形状**：全反汇编对该段的访问都是 `$0300 + 槽号*16 + 偏移` 的**槽步长 16**形状（`Objects.0.status,x` 配 `x = $00,$10,…,$F0`；例：`prg7_engine.asm:3703-3716` 用 `#SamusProjectiles.0 - Objects` 作 x）。没有「区号 × 基址」的索引。
2. **清零循环**：`LC836` 按 `status` 步进清到 `$03FE`，是池清零，不是「按区分段清」。
3. **运行分布**：用区号应能预测哪些字节活跃（Brinstar 与 Ridley 房活跃段不同）；实测是**同一批槽**跨区活跃（门槽、子弹槽在五区都用），差异只由「该房有没有门/道具/雕像」决定。

结论：这段的语义**完全由对象池槽位决定**；卡面里的「每区状态数组尾」应撤回。为避免后续卡再按区猜，建议把本结论写进 `docs/design/typed-struct-plan.md` 的 B2-a 条目。

## 7. 与 Rust 现有实现的逐项对照

| 项 | 位置 | 状态 |
|---|---|---|
| 池基址/步长/槽数 | `crates/game-core/src/world/object.rs:11/13/15` | 一致 |
| `Object` 字段偏移 | `world/object.rs:20-44`（`obj::STATUS…HI`） | 一致；`data07/data0F` 的多义已在注释里点出 |
| 槽借用视图 | `world/object.rs:47-63`（`Manager::object(slot)`，越界 `assert`） | 一致（fail-loud 优于原版越界） |
| 槽 2 电梯 | `world/elevator.rs:24` `ELEVATOR = 0x0320` | 一致（含 `ElevatorType` 语义） |
| 槽 6-7 雕像 | `world/statue.rs:40-50`（`STATUE_SLOT/Y/X`、`KRAID_RAISE/RIDLEY_RAISE`、`KRAID_IS_HIT/RIDLEY_IS_HIT`、`KRAID_Y/RIDLEY_Y`） | 一致 |
| 槽 8-11 门 | `world/room_draw.rs:276/550`（`OBJECTS_BASE`/`DoorHitPoints`） | 一致 |
| 槽 13-15 子弹 | `samus/handler.rs:660` `base = 0x03D0`，逐槽 `+s*0x10` | 一致 |
| 槽 1 头 7 B = Samus 扩展 | `ram_map.rs:334-381` | **一致但未标注与对象池重叠**；建议在 `object.rs` 文档注释里写明 |
| 槽 1 尾 9 B / 槽 3 / 槽 5 / 槽 12 | 无任何具名常量 | **缺失**（建议显式标「预留，恒 0」，避免下一个人按地址命名） |
| `$0324 PasswordStat00` | `title/mod.rs:1473` 写 0 | 一致；**语义未取证**（原版注释即「无功能」），只能留 `_todo_` |
| `$0331` | `sound/music.rs:122` `music_schedule()` 读 `ram[0x0331]` | **无据**：asm 零引用、零生产调用者（只有自测），建议实现卡删或换来源 |

## 8. typed 化建议（供实现卡，不是本卡的结论）

| 字节 | 建议 | 理由 |
|---|---|---|
| 全段 `$0317-$03FF` | **用对象池 typed 视图表达**（已有 `Manager::object(slot)`），不要新建「状态面尾部」结构 | §1/§6：语义=槽位 |
| 槽 1 头 7 B | 保留 `SAMUS_*` 常量，但在池模型里**显式声明被覆盖**（注释 + 一条单测断言 `object(1)` 不用于 Samus 物理） | §3 |
| `$0317-$031F`、`$0330-$033F`、`$0350-$035F`、`$03C0-$03CF` | 可 typed 成「预留槽」，但**字段名只能是 `_reserved`**，并在文档写「原版未使用，禁止按地址命名」 | §5 |
| `$0324` | 留 `_todo_`，不加语义名 | 原版注释明说不明功能 |
| `$0331` | 不作为可靠语义；删 `music_schedule()` 或补真实来源 | asm 零引用 |
| `$03FF` | 保留 off-by-one 行为（`ClearObjectStatus` 不清它）；typed 层不要顺手「修好」 | `prg7_engine.asm:1866` 注释即 off-by-one bug，Stage A 保真 |

## 9. 复现命令（逐条本机实跑）

```bash
# 1) 符号表：16 槽别名与地址（输出见 §1）
grep -E " (Samus|Elevator|Statue|Objects|Object031[0-9]|Object0330|PowerUpDraw|Object0350|Object0370|Doors|Object03C0|SamusProjectiles)$" \
  refs/m1disasm/out/M1_NES_NTSC.sym
```

```bash
# 2) Object 字段表与「Samus RAM ($0300-$0316)」注释（输出见 §1/§3）
sed -n '217,239p;882,901p' refs/m1disasm/SRC/constants_ram.asm
```

```bash
# 3) 池清零与 off-by-one（输出见 §1）
grep -rn 'Clear RAM \$300 thru \$3FE' refs/m1disasm/SRC/*.asm
```

```python
# 4) 运行期逐槽活跃度（11 116 份 trace，输出见 §5）
import glob, pathlib, collections
nz = collections.Counter(); tot = 0
for p in sorted(glob.glob('analysis/traces/*.ram.hex')):
    b = bytes.fromhex(pathlib.Path(p).read_text().strip())
    tot += 1
    for a in range(0x317, 0x400):
        if b[a]: nz[a] += 1
print('files', tot)
for a in range(0x310, 0x400, 0x10):
    print(f'{a:03X}', sum(nz.get(a + i, 0) for i in range(16)))
```

```bash
# 5) 槽 1 假别名与 $0331 零引用（输出见 §2/§7）
grep -rn 'Object0310' refs/m1disasm/SRC/*.asm
grep -rn '\$0331' refs/m1disasm/SRC ; echo "0331 hits: $?"
grep -rn 'music_schedule' crates/ --include=*.rs
```

## 10. 假设区（C 级）与未覆盖

- **C1**：槽 7 只用了 `$0370`（`RidleyStatueY`），`$0371-$037F` 在运行期偶有非零（数十帧），可能是雕像绘制时的临时 y/x 写入；未逐帧核对写者，标 C。
- **C2**：槽 12（`$03C0`）在少数帧非零，可能与「Mellow/爆炸绘制」的临时槽复用有关，未取证。
- **未覆盖**：
  - 未逐条列出 16 槽的所有写者（本卡给的是代表写点；全量写点清单应在实现卡用 RAM 写钩子 trace 生成）。
  - 未核对 `$0320-$032F` 在**标题态与游戏态切换那一帧**的残留值（切换边界是否有脏读）。
  - 未核对 `$03FF` 漏清在长跑中是否积累可见差异（需要 >10 万帧 trace）。

## 11. 证据指针

| 事实 | 证据 |
|---|---|
| 16 槽别名/地址 | `refs/m1disasm/out/M1_NES_NTSC.sym`（wla 导出） |
| `Object` 字段与偏移 | `refs/m1disasm/SRC/constants_ram.asm:217-239` |
| Samus 扩展面 `$0310-$0316` | 同上 `:882-901`；`crates/game-core/src/ram_map.rs:334-381` |
| 电梯槽字段与写值 | `refs/m1disasm/SRC/prg7_engine.asm`（`SpawnElevatorRoutine`/`UpdateElevator`）；本仓库 `world/elevator.rs:24` |
| 密码态字段 | `constants_ram.asm:950-958`；写点 `prg0_title.asm` |
| 池清零 off-by-one | `prg7_engine.asm:1866` |
| 槽活跃度统计 | §9 命令 4，11 116 份 `analysis/traces/*.ram.hex` |
| Rust 池模型 | `crates/game-core/src/world/object.rs:11-63` |
| `$0331` 无据读取 | `crates/game-core/src/sound/music.rs:122`；`grep -rn '\$0331' refs/m1disasm/SRC` 零命中 |
