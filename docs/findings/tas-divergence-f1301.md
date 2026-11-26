# TAS 新首分歧 f1301：`$0454`（`Ens[5].isHit`）emu `$00` / rust `$20`

> 卡片：#120。判等面：`$0400-$04FF`（`Ens` 6 槽×16 B）+ `$0300-$03FF`（对象池）+ 帧计数。
> 运行环境：`tools/tas_replay_diff.py`（emu 侧 Mesen2 2.1.1 + `lordtom-metroid-100p` 回放，rust 侧 `game-shell-pc` 同步喂键）。
> 取证工件：`/tmp/a2-fresh/`（`coverage.json`，`at=2026-11-26T18:00:14Z`，`head=811bbbca`，range `0400-04FF`，30000 帧）与 `/tmp/d120/`（range `0300-04FF`，1400 帧，`at=2026-11-26T18:38:30Z`）。

## 0. 结论（二选一）

**A：实现差（Rust 侧多判了一次武器命中），不是测量差。**

- 判等相位一致：两侧 `f1300` 的 `$0300-$04FF` 512 B 逐字节相同（含敌槽 5、弹槽 13 的位置/状态/血量）。
- `f1301` rust 侧在同一次帧内改了三处状态：敌槽 5 `isHit←$20`、`weaponAction←$01`（`wa_RegularBeam`）、弹槽 13 `isHit←$20`；emu 侧同一帧这三处全为 0；在 1–1905 全段 emu 的 `$0454/$045E` 均为 0（emu 首次真正置位 slot5 命中位是 **f2157**，`$0454=0x38`、`$045E=0x01`，见 §2f）。
- FC（`$2D/$2E`）两侧 f1298–f1305 逐帧相同（`250/123 → 0/17`），排除了「掉帧/多跑一拍」这类测量差。
- 分歧后果在下一个帧可见：`f1302` rust 弹槽 13 `status=4`（BulletExplode）、位置 `2A/1D`，emu 仍 `status=1`、位置 `28/1B` ⇒ rust 把子弹打爆了，emu 没有。

**未收敛的部分（C 级，留给实现卡）**：ROM 的判盒（`F1BF/F1FA`）按位置/半径和看，`f1301` 这一对理应判中（`dy=7 < 8+2`、`dx=6 < 8+2`），但 emu 在 1–1905 帧内没有任何命中痕迹（血量 1、`prevStatus=0`、`weaponAction=0`；首次命中在 f2157）。两种候选机制都还没被单独证伪：
1. **判盒取样位置不同**（Rust 用的是移动后位置，ROM 可能在移动前）：`f1300` 弹 y=`$A5` 与敌 y=`$9A` 差 11 ≥ 10 不碰；`f1301` 弹移到 `$A1` 后才差 7。Rust 日志显示它是在移动后判的；ROM 的 `UpdateWorld 07:CB29` 件序里 `CollisionDetection` 也在 `UpdateSamusProjectiles` 之后，所以这条只能算「可能」。
2. **判盒条件本身不同**（半径和/边界/`ScrollDir` 分支）：Rust 用 `unsigned_abs` 对称化，ROM 的 `CheckCollisionOfXSlotAndYSlot 07:F1FA` 走 Carry 比较且横向房不做 `SCRN_VY` 补偿；数值上两者在本例应同判「碰」，所以这条也未被证实。

推翻方法（实现卡按这个顺序做）：在 emu 侧对 `07:F2CA`（`CollisionDetectionEnemy_ReactToCollisionWithSamusProjectile`）下读断点，记录 `f1301` 是否命中槽 5/弹 13；若命中但帧末 `$0454=0`，说明 ROM 在命中后同一帧又清了位（则分歧回到「清位时序」）；若未命中，再逐条比对 `F1FA` 的入参（`Temp00_Diff`/`Temp04`/`Temp05`/`Temp10`）。

## 0.1 复现（干净 HEAD，双腿重建）

```bash
cargo build -q -p game-shell-pc
python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 30000 --range 0400-04FF --out /tmp/f1301-clean --allow-rust-missing
```
实测（`/tmp/f1301-clean/report.json`）：
- `compared=29936`，`diverge=28659`；
- `first_divergence = {frame:1301, field:"$0454", offset:84, emu:0, rust:32, diff_bytes:2}`（与卡面一致）；
- **f1–f1300 该面 1277 个可比帧全 equal**（`diff<=1300` 计数 0），首差就是 f1301；f1301 本身 2 B（`$0454/$045E`），f1302 起扩到 8–12 B（`$0454/$045E` + 弹槽 13 的 status/位置连锁，见 §1）。

> 说明：`--allow-rust-missing` 只用于让启动期 25 个 span 帧记「不可比」，不影响 f1301 的判定（这些帧在 f1–60 内）。未使用 `--resume`。

## 1. 田野数据（f1298–f1305）

`s` = `$0300` 基址，`e` = `$0400` 基址。emu 列取自 `/tmp/d120/emu`；rust 列取自 `/tmp/d120/rust/full.state.jsonl`（`iram` = `$0100-$07FF`）。

| 帧 | 敌槽 5 `$0450-$045F`（emu / rust） | 弹槽 13 `$03D0-$03DF`（emu / rust） | 差异字节 |
|---|---|---|---|
| 1298–1300 | 全等 `9A78 0000 0058 0000 0000 0102 0001 0000` | 全等 `0102 0228 011B 1C00 FC00 0001 01A5 7E09` | 无 |
| 1301 | `9A78 0000 0058 0000 0000 0102 0001 0000` / `9A78 0000 2058 0000 0000 0102 0001 0100` | `…A1 7E08` / `…20 01 A1 7E08`（`$03DA←20`） | `$03DA $0454 $045E`（3 B） |
| 1302 | 同上 emu / 同上 rust | emu `0102 0228 … 28 1B`（status1）/ rust `0400 002A 011D …`（status4） | 8 B（`$03D0-$03D6` + 上面 2 B） |
| 1303 | emu 不变 / rust 不变 | emu status1 / rust status4 | 12 B（多 `$0303/$0306` Samus 动画瞬态） |
| 1304 | 同 1303（rust `$0454` 仍 `20`、`$045E` 仍 `01`，**没有回落**） | rust status4 继续 | 10 B |
| 1305–1308 | rust `$0454=20/$045E=01` 持续（实测至 f1905，见下） | rust status4 | 2 B（`$0454/$045E`） |

要点：
- **持久分歧是两字节 `$0454=20`（`isHit` bit5）与 `$045E=01`（`weaponAction`=1=`wa_RegularBeam`）**，正是 ROM `07:F2CA` 在同一段里写的那两个值（`:10567` 武器动作、`:10575` isHit）。实测 `$0454` 的 bit5 在 rust 侧**从 f1301 一直保持到 f1905**：该区间内 `rust.state.jsonl` 有采样的 601 帧**全部**是 `0x20`；`f1309/f1310/f1572/f1573` 在这份工件里**根本没有记录**（不是被清零），此前「三段连续、两处 2 帧空档」是把缺帧读成清零的错误（见本卡退回 B2 的更正）。emu 在 1–1905 全段 `$0454` 恒 0。
- `f1303` 的 `$0303/$0306` 是 Samus 槽动画索引的 1 帧瞬态（与 `f1297` 同源），不是新分歧。
- **更早的瞬态**（`$0300` 面）：`f484` 有 6 B 单帧差（`$0341-$0343/$034C-$034E`，对象池槽 4 动画索引，emu 非零 / rust 0），下一帧自愈；`f1297` 有 2 B 单帧差（Samus `$0303/$0306`）。两者都只存在一帧，属采样相位抖动，不是持久分歧。

## 2. `$0454` 的名字与写者

- `$0454` = `Ens.0.isHit`，槽 5（`$0400 + 5*16 + 4`）。`isHit` 位义（`constants_ram.asm:90-95`）：bit5 = 「hit by samus weapon (projectile or screw attack)」，bit4 = 从上方命中，bit3 = 从左侧命中。`$20` 就是「被武器命中、无方向位」。
- `$045E` = 同槽偏移 `0x0E` = **`Ens.0.weaponAction`**（「击杀它的武器类型」），ROM 侧由 `07:F2CA` 的 `:10567 sta Ens.0.weaponAction,x` 写（字节 `9d 0e 04`，地址 `$040E+x` 已按 ROM 复核）。`$01` = `wa_RegularBeam`（`constants.asm:117`）。

  > 【2026-11-27 更正】本文原写「`$045E` = `prevStatus` = `enemyStatus_Resting`」，**错**：`prevStatus` 在偏移 `0x0C`（`$045C`），由 `EnemyReactToSamusProjectile 07:F536` 写（ROM 字节 `9d 0c 04`）；`0x0E` 是 `weaponAction`。两处均由 ROM 原始字节复核，Rust 常量 `ens::PREV_STATUS=12` / `ens::WEAPON_ACTION=14`（`enemy/state.rs:34/37`）与之一致。
- **写者链（A 级）**：`UpdateWorld 07:CB29` → `jsr CollisionDetection 07:F034` → 敌×弹环 `07:F09D`（`prg7_engine.asm:10134-10168`：`ldx #$50` 从槽 5 起、`jsr Xminus16` 到槽 0）→ `CollisionDetectionEnemy_CheckWithObjectYSlot 07:F140`（判盒）→ **命中** `CollisionDetectionEnemy_ReactToCollisionWithSamusProjectile 07:F2CA`：`Ens.weaponAction←弹 status`、`SetSamusProjectileIsHit 07:F279`（弹 `isHit |= $20`）、`GetEnemyIsHitFlags 07:F332`（`(Temp10^3)<<3`）`ora Ens.isHit`。`prevStatus` 由后续 `EnemyReactToSamusProjectile 07:F536` 写（`:10603` 一带）。
- Rust 对应件：`crates/game-core/src/weapon/projectile.rs:502 uw_collision_detection`（判盒在 `:529-541`，命中面在 `:543-556`），`crates/game-core/src/enemy/framework.rs:177 enemy_react_to_projectile` 写 `prevStatus`（`:189`）。


## 2b. `$0450-$0457` 邻域逐字节（`Ens` 槽 5 = `$0450`，步距 16 B）

`.struct En` 字段序见 `refs/m1disasm/SRC/constants_ram.asm:85-121`；`Ens` 是 6 槽实例（`constants_ram.asm:1294` 一带）。**这 8 个字节在 asm 里全部有具名字段，没有 `_todo_`**：

| 地址 | 偏移 | asm 名 | 语义（asm 注释） | ROM 写点（bank:addr） | f1301 两侧值 |
|---|---|---|---|---|---|
| `$0450` | +0 | `Ens.0.y` | 敌人在房间内的 y（非屏坐标） | 移动/生成例程（`UpdateEnemy_ForceSpeedTowardsSamus` 等） | `9A` / `9A` |
| `$0451` | +1 | `Ens.0.x` | 敌人在房间内的 x | 同上 | `78` / `78` |
| `$0452` | +2 | `Ens.0.speedY` | y 速度（MSB=上） | `ApplySpeedToPosition` / 各 AI | `00` / `00` |
| `$0453` | +3 | `Ens.0.speedX` | x 速度（MSB=左） | 同上 | `00` / `00` |
| `$0454` | +4 | `Ens.0.isHit` | bit5=被 Samus 武器命中；bit4=从上方；bit3=从左侧；bit2=碰到 Samus | `07:F2CA`（`prg7_engine.asm:10561-10576`：`sta Ens.0.weaponAction,x` 在 `:10567`、`sta Ens.0.isHit,x` 在 `:10575`）+ 清位 `UpdateEnemyCommon@clearIsHitFlags 07:F42D`（`:10825-10830`） | `00` / **`20`** |
| `$0455` | +5 | `Ens.0.data05` | bit0 朝向、bit1 可见、bit2 朝向上下、bit3 激活条件、bit4 距离阈值、bit5 休息触发、bit6 30FPS 切换、bit7 朝向改用 bit2 | 各 AI 例程（爬行族 `UpdateEnemy_Crawler`） | `58` / `58` |
| `$0456` | +6 | `Ens.0.chunkyExplosionTimer` | 大爆炸计时 | `ExplodeEnemy 07:F611` 起 | `00` / `00` |
| `$0457` | +7 | `Ens.0.movementInstrIndex` | 移动指令表索引 | `EnemyMove 07:8058` | `00` / `00` |

同槽的 `EnsExtra`（基址 `$6AF4`，槽 5 = `$6B44`）在 f1301 两侧全等：`status=2(Active)`、`radiusY=8`、`radiusX=8`、`animFrame=0x5F`、`animDelay=1`、`resetAnimIndex=0x3E`、`animIndex=0x40`、`hi=0`、`type=0x05`（Brinstar 爬行敌）、`data1F=0`。⇒ 分歧不是「敌人身份/血量/位置」造成的，只落在 `isHit`（`$0454`）与 `weaponAction`（`$045E`）两位——也就是 `07:F2CA` 命中段一次写下的那两个值。

## 2c. Rust 侧写点（首个分歧帧的写入路径）

- `crates/game-core/src/weapon/projectile.rs:502 uw_collision_detection`（ROM `CollisionDetection 07:F034` 的敌×弹环）：
  - `:544` `mem.write_ram(0x0400 + x + 0x0E, pstat)` ← `weaponAction`（f1301 写 **`$045E=01`**；ROM 对应 `:10567` 的 `9d 0e 04`）；
  - `:545` `mem.write_ram(py + 0x0A, … | 0x20)` ← 弹 `isHit`（f1301 写 `$03DA=20`）；
  - `:556` `mem.write_ram(0x0400 + x + 4, cur | flags)` ← 敌 `isHit`（f1301 写 `$0454=20`）；
  - 判盒在 `:529-541`（`dy=(ey-py_y).unsigned_abs()`、`dx=(ex-py_x).unsigned_abs()`，`dy>=ery+pry || dx>=erx+prx` 判不碰）。
- `crates/game-core/src/enemy/framework.rs:177 enemy_react_to_projectile`（ROM `EnemyReactToSamusProjectile 07:F536`）：
  - `:189` `mem.write_ram(0x0400 + px + 0x0C, (st0 & 0x3F) | (sa & 0xC0))` ← `prevStatus`（`$045C`；f1301 两侧都还是 `00`，见 §2f）；
  - `:215` `set_status(EnemyStatus::Hurt)`、`:220` `set_health(nhp)`、`:223` `explode_enemy`。
- 清位：`framework.rs:776` `set_is_hit(0)`（`UpdateEnemyCommon@clearIsHitFlags 07:F42D`，ROM `:10825-10830` 同时清 `isHit` 与 `weaponAction`）。它是**逐帧**清的；实测 rust 的 bit5 仍在 1301–1905 连续存在，说明同一段碰撞环在这 605 帧里**每帧重新置位**（清位后又被置），不是「只亮 3 帧」。
- 端口状态：这条路径**已端口**（不是 `porting-backlog.md` 的缺口）；差异落在判盒的取样/条件，不在「有没有实现」。

**调用序核对（A 级）**：ROM `UpdateWorld 07:CB29`（`prg7_engine.asm:2494`，`jsr CollisionDetection` 在 `:2512`）先 `UpdateAllEnemies`→`UpdateSamusProjectiles`→`UpdateSamus`→…→`CollisionDetection`（最后一件），rust `world/update.rs:100 update_world` 的件序同样把 `uw_collision_detection` 放在最后一组（`:147`）。所以两侧都是「先动、后判」，**不能用「一先一后」解释**。f1301 rust 只改了两个字节（`$0454`、`$045E`），都在 `CollisionDetection` 的命中段里（`F2CA`），**没有** `EnemyReactToSamusProjectile` 的痕迹（`$045C` 两侧仍是 `00`）——说明分歧发生在「判中」这一步，react 之后的伤害/受伤链在 f1301 根本还没开始跑。

## 2d. 可证伪预测（判据写死，失败即否证）

**P1（短程，机制推出的下一处新地址）**：若 f1301 的根因是「rust 在碰撞环里多判中一次武器命中」，则后续新分歧应呈「同一 `Ens` 槽的 `isHit(+4)` 与 `weaponAction(+0xE)` 成对出现（`F2CA` 正是同时写这两字节）、随后 3 帧内该槽其它字节被改写」的模式。据此预测：**f1301 之后第一处此前未出现过的分歧地址是 f1308 的 `$0424`（槽 2 `isHit`）与 `$042E`（槽 2 `weaponAction`），并在 f1311 出现该槽 `$0420-$042F` 的多字节改写。**

- 判据：在 2200 帧工件（`/tmp/f122`）里若 1302–1307 出现任何新地址，或 f1308 这两字节不相差，或 f1311 无槽 2 改写 ⇒ P1 否证，根因需重定。
- 实测结果：**命中**——新地址序列前四组正是 `(1301,$0454/$045E) → (1308,$0424/$042E) → (1311,$0420/$0421/$0429/$042B/$042C/$042F)`，与预测一致（`/tmp/f122` 与 30000 帧工件 `/tmp/f1301-clean` 同形）。

**P2（给下一张修复卡的前瞻判据）**：若修复的是判盒（取样相位或条件），重跑 `--frames 2200` 后 `first_divergence` 应**向后移动**（不再是 f1301），且 f1302 起弹槽 13 不再 `status=4`；若仍报 f1301 `$0454`，则本卡根因判错，修复方向作废。

- 判据：`python3 tools/tas_replay_diff.py replay --frames 2200 --range 0400-04FF` 的 `first_divergence.frame`；等于 1301 ⇒ P2 否证。此条留待修复卡执行，本卡不施工。

## 2e. 复现（本卡 #122 口径）

```bash
git rev-parse --short HEAD        # a61a84e2
python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 2200 --range 0400-04FF --out /tmp/f122 --allow-rust-missing
```
读数：`compared=2169 / diverge=892 / first={frame:1301, field:"$0454", offset:84, emu:0, rust:32, diff_bytes:2}`。
工件路径：`/tmp/f122/emu-traces/tas-lordtom-metroid-100p-2200.frames.jsonl`（emu，键 `ens`=`$0400-$04FF`）、`/tmp/f122/rust.state.jsonl`（rust，键 `iram`=`$0100-$07FF`，`$0454`→`iram[0x354]`）；30000 帧对照 `/tmp/f1301-clean/`（`compared=29936`，首分歧同帧同地址）。


## 2f. 后续两类不同现象（与 f1301 分开记）

1. **`prevStatus`（`$045C`）的分歧出现得晚**：`$045C` 在 1301–2157 两侧都相等；直到 **f2158** 起才有 41 帧 emu=`02` / rust=`00`（`/tmp/f122` 逐帧解）。这与 f1301 的命中位不是同一件事，说明 rust 在更晚的某次受击里少写了一次「受击前状态」，需要单独定位（不属本卡结论）。
2. **f1906 之后槽 5 在两侧已不是同一实体**：f1904/1905 时 emu 槽 5 是 `y=0x38,x=0xAC`（`data05=0x18`），rust 仍是 `y=0x9A,x=0x78`——两边对象池分配已分叉，此后 `$0454` 的对比不再有意义。故本文所有「持续到 f1905」的读数都以此为界；f2157/f2171 那两个「emu 非零 / rust 0」的孤立帧不并入本卡的命中位分歧。

## 3. 三面对照

| 面 | f1298–f1300 | f1301 | f1302–f1305 | 说明 |
|---|---|---|---|---|
| 字段（`$0300-$04FF`） | 全等 | 3 B 差（`$03DA/$0454/$045E`） | 8–12 B，含弹爆 | 见 §1 |
| FC（`$2D/$2E`） | 全等 | 全等 `253/198` | 全等 `0/17…` | emu `analysis/traces/fc30000.frames.jsonl`；rust `zp` 列 ⇒ 排除掉帧 |
| OAM（`$0200-$02FF` 精灵页） | 全等 | 3 B 差 | 9/34/12/59 B 差（随弹爆炸扩散） | 实测（`/tmp/d1301-oam`，`--range 0200-04FF`）：**该面 1375 帧里 107 帧本就不等**，首差 `f3` 96 B（`$0200` emu `$F4` vs rust `$F0`，尾清相位），`f7/f8/f9/f10` 同源 ⇒ 这是既有绘制/清页相位差，不是本卡新分歧。f1298–f1300 恰好全等，f1301 的 3 B 与字段面同字节；f1302 起弹爆使精灵消失，差值扩到 9/34 B。**结论：OAM 面当前不能当独立判据**（既有 107 帧瞬态未清），但它在 f1298–1300 与「弹炸掉之后」的表现与字段面一致。 |

## 4. 复跑命令

```bash
# 1) 生成两侧逐帧（1300 帧足够覆盖 f1301）
python3 - <<'PY'
from tools.tas_replay_diff import main
PY
python3 tools/tas_replay_diff.py --movie movies/lordtom-metroid-100p.inputlog \
  --frames 1400 --range 0300-04FF --out /tmp/d1301
# 2) 只看 emu：$0454/$045E 在 1..1400 是否非零
python3 - <<'PY'
import json
n=0
for l in open('/tmp/d1301/emu/seg-000000-001400.jsonl'):
    o=json.loads(l); e=bytes.fromhex(o['w']['ens'])
    if e[0x154] or e[0x15E]:
        n+=1; print(o['f'], hex(e[0x154]), hex(e[0x15E]))
print('emu 槽5 命中/前态非零帧数 =', n)
PY
# 3) 取 OAM 面（需要时；须 --allow-rust-missing，否则启动期 span 帧会 fail-loud）
python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 1400 --range 0200-04FF --out /tmp/d1301-oam --allow-rust-missing
# 期望：compared 1375 / diverge 107 / first_divergence f3 $0200 96B（既有绘制相位差）；
#        f1298–f1300 equal，f1301 diff_bytes=3
```

期望读数（本卡实测）：步骤 2（只统计 1–1400）打 `emu 槽5 命中/前态非零帧数 = 0`；若扫到 2200 帧，则 emu 首次命中在 f2157（`$0454=0x38/$045E=0x01`），不属于 f1301 窗口；`/tmp/d1301/rust/full.state.jsonl` 的 f1301 `iram[$0454-0x100]=0x20`、`iram[$045E-0x100]=0x01`。

## 5. 对后续卡的影响

- 「emu↔rust 逐帧一致」在 f1301 之前成立（`$0400-$04FF` 面 1..1300 逐字节等，除 f484/f1297 两个单帧瞬态）；从 f1301 起这条长片不再能当等价证据。
- 本卡的 A 级结论只到「Rust 多算一次命中、并且把弹打爆」；**命中判定为什么在 emu 侧没有发生**仍未收敛（§0 的 C1/C2）。实现卡应先做 §0 的断点验证再改判盒，不要直接照「对称化」或「相位」其中一种改。

## 6. C 级假设清单

- C1：ROM 判盒使用的是「移动前位置」。推翻方法：在 `07:F140` 入口记录 `Temp07/Temp06/Temp09/Temp08`，比较与帧末位置是否一致。
- C2：ROM 对 `wa_Bullet`（status 1）有额外免疫/跳过条件（例如该帧弹已被标记过）。推翻方法：在 `07:F0B4` 环里记录 `y` 与 `Objects.status`，统计该弹被检查的次数。
- C3：`f484`/`f1297` 的瞬态是 emu 采样点与 rust 采样点的半帧相位差（不是行为差）。推翻方法：同帧采集两侧 `$0300/$0303/$0306` 的写入时刻（Mesen 写回调 vs rust 每步快照）。

## 7. 回归（本卡只写文档，代码零改动）

| 项 | 读数 |
|---|---|
| `cargo test -q --workspace` | **1250 passed / 0 failed**（14 target 合计） |
| `python3 gates/cli.py check` | **PASS=24 FAIL=0 SKIP=3 TOLERATED=1**（8.22s） |
| `python3 tools/l4_diff_run.py run-all --skip-emu` | **21 案 / 172 格 · match 171 · diff 1 · blocked 0**；`shots_rust/matrix.json` 本次写入（`at=2026-11-26T19:07:18Z`、`head=a61a84e2`），唯一 diff 仍是 `case-mechanism-input-conflict` f674=84px（既有伪差） |

本卡不动 `crates/`、`tools/`、`data/status/`，上述数字用于证明判据未被放宽。
