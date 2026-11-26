# f1551 `$040D` 首分歧定界：掉落门链缺口的下游症状

> 卡 #140（tsk_01M8DEZMJWYZWDJ1T3J6RE），只写本文件，`crates/` 零改动。
> 前置：#137（判盒重建）与 #138（弹槽 typed）已在 HEAD。f1301 `$0454` 已不再出现，首分歧后移到 f1551。

## 0 结论

**f1551 的 `$040D` 不是判盒（`$0454/$045E`）链的下一站，而是「敌死亡→掉落物」门链缺口的第一个被观测面。**

- `$040D` = `Ens.0.data0D`（符号表 `refs/m1disasm/out/M1_NES_NTSC.sym:11127`），语义随状态：
  Resting=「强制朝 Samus 加速」的延迟、Frozen=冻结计时（×8）、**Pickup=拾取物消失计时（×4）**。
- f1550 有一个敌在 slot0 死亡，死亡码先写 `status←Pickup`、`data0D←$60`，**再**过三道掉落门；emu 在「大能量资格」这一道失败 ⇒ `LDD5B` ⇒ `RemoveEnemy`（status←0，`data0D` 残留在 `$60`）；rust 没有实现这几道门 ⇒ 保持 `status=Pickup`，于是从 f1551 起按 `FrameCount & 3 == 0` 每 4 帧递减 `data0D`。
- 因此 f1551 的 1 B 差（emu `$60` / rust `$5F`）是**下游计时器有没有开始走**，不是值的写入不同；两侧 RNG 在此帧相同（`RandomNumber1 = $E5`）。
- 真正的首差发生在 f1550 的 **非判等面**：`EnsExtra.status`（emu 0 / rust 5）与 `animFrame`（emu 保留 `$37` / rust `$89`）。判等面 `$0400-$04FF` 要到 f1551 才看得见，因为 status 在 `$6AF4` 区。

## 1 复现（干净 HEAD）

```bash
cd /home/peng/workspace/metroid-decomp
git log --oneline -1            # ee7c83b7/#137、fb77fe65/#138 均已在 HEAD
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog --frames 4000 \
  --range 0400-04FF --out /tmp/f1551 --allow-rust-missing
# 期望：compared=3967 diverge_frames=2442
# first={"frame":1551,"field":"$040D","offset":13,"emu":96,"rust":95,"diff_bytes":1}
```

判等面帧哈希（`/tmp/f1551/report.json`）：

| 帧 | 相等 | sha256(emu) | sha256(rust) |
|---|---|---|---|
| 1549 | ✓ | `42a9320daf4a0985…` | `42a9320daf4a0985…` |
| 1550 | ✓ | `66392034f3d416a4…` | `66392034f3d416a4…` |
| 1551 | ✗ | `c0c4dc85ec789e21…` | `3229da7887fa2850…` |

## 2 `$040D` 的语义与读写点

符号表（`out/M1_NES_NTSC.sym`）片段：`$040B health`、`$040C prevStatus`、**`$040D data0D`**、`$040E weaponAction`、`$040F specialAttribs`。所以：

- **不是** health（`$040B`）、**不是** prevStatus（`$040C`）、**不是** weaponAction（`$040E`）——这三者上一轮 f1301 定界里都出现过，容易混。
- 语义（`refs/m1disasm/SRC/constants_ram.asm:118-121`）：Resting=force speed towards samus delay；Frozen=frozen timer（×8）；Pickup=die delay（×4）。

写点（bank:addr / asm 行）：

| 场景 | 位置 | 说明 |
|---|---|---|
| 敌死亡转拾取 | `07:DD2A`（`prg7_engine.asm:5976`） | `data0D←$60`（384 帧上限），在掉落门前写 |
| 拾取计时 | `07:F4D9`（`:10960`） | `FrameCount&3==0` 时 `dec`，减到0 `RemoveEnemy` |
| 冻结 | `07:F458`（`:10861`） | `FrameCount&7==0` 时递减 |
| 解冻回填 | `07:F46A`（`:10874`） | `EnemyForceSpeedTowardsSamusDelayTbl[type]` |
| 敌初始化 | `07:F860`（`:11712`） | 同表，出生态 |
| 强制加速度计满 | `07:F6C9-F6D0`（`:11345/11352`） | `InitEnActiveAnimIndex` 径 |
| 冰冻法术 | `07:F562`（`:11064`） | `data0D←$40` |

读点：`07:F471`（`:10877`，冻结闪烁阈值 `<$0B`）、`:10960` 的 `dec/bne`、`:11348`、`:10975`（拾取消失判定）。

## 3 f1550–f1551 的实际路径

`EnemyCompleteDeath ; 07:DD0D` 的顺序（`prg7_engine.asm:5957-6032`）：

1. `status←Pickup`（`07:DD20`，`:5971`）；
2. `data0D←$60`（`07:DD2A`，`:5976`）；
3. `RandomNumber1`：`cmp #$10 / bcc LDD5B`（1/16 无掉落）；
4. `and #$07` → `PickupTypeTable[7]`（`07:DE35` = `80 81 89 80 81 89 81 89`）；
5. 按掉落型过配额：missile 需 `MissilePickupQtyCur != Max` 且 `MaxMissiles != 0`；energy 需 `EnergyPickupQtyCur != Max`（**先 `inc EnergyPickupQtyCur`，再判大能量资格**）；
6. 大能量还要 `lsr L968B[type] / bcs`（bit0=1 才有资格）；失败 → `LDD5B`（`:6017`）→ 非 Tourian ⇒ `RemoveEnemy`（`:6022`）。

本例两侧实测（emu 侧 watch `6AF4-6B04` 的临时场景 + `analysis/traces/*.frames.jsonl`；rust 侧 `rust.state.jsonl` 的 `zp/wram/iram`）：

| 帧 | RandomNumber1 | 选型 r&7=5 | emu `EnsExtra.status` | emu animFrame | rust status | rust animFrame | `$040D` emu/rust |
|---|---|---|---|---|---|---|---|
| 1548 | `B4` | — | 2 | `5D` | 2 | `5D` | `01/01` |
| 1549 | `CC` | — | 2 | `5D` | 2 | `5D` | `01/01` |
| 1550 | **`E5`** | → `$89`（BigEnergy） | **0** | `37`（旧值） | **5** | **`89`** | `60/60`（相等） |
| 1551 | `FE` | — | 0 | `37` | 5 | `89` | **`60/5F`（首分歧）** |

关键佐证：

- `r&7=5` ⇒ `PickupTypeTable[5]=$89` = BigEnergy。之后 emu 的 `EnergyPickupQtyCur($96)` 从 0 变 1（f1550，先 inc 再判资格），但**最终 status=0** ⇒ 资格那道没过。
- 该敌死亡前的 `type=$20`；`L968B[$20]`（bank1 `01:968B`）实测 `00` ⇒ bit0=0 ⇒ 大能量不合格 ⇒ `LDD5B`。**这是 A 级字节证据**。
- rust 完全没有实现 3–6 步：它只读 `RANDOM1`、判 `<0x10`，然后直接写 `type/animFrame` 并返回（`framework.rs:583-604`），配额、`MaxMissiles==0`、`L968B` 资格都缺。

## 4 Rust 侧写点与首个分歧帧的路径对照

| 行为 | ROM | Rust |
|---|---|---|
| `status←Pickup` | `07:DD20` | `crates/game-core/src/enemy/framework.rs:583` |
| `data0D←$60` | `07:DD2A` | `framework.rs:584` |
| 1/16 无掉落 | `07:DD2E` | `framework.rs:585-589` |
| 掉落型 `PickupTypeTable[r&7]` | `07:DD30` | `framework.rs:602-604` |
| 导弹/能量配额 | `07:DD34-DD5A` | **缺**（注释自认「qty 限域=分卡（记账）」，`framework.rs:563`） |
| 大能量资格 `L968B&1` | `07:DD4F-DD55` | **缺** |
| 拾取计时递减 | `07:F4D9` | `framework.rs:501-508`（`FrameCount&3==0`） |

⇒ f1551 的差路径：`frame_end(敌死)` → ROM 走 `LDD5B` 把 status 清 0；rust 留在 Pickup → 下一帧 `update_enemy_pickup` 的 `FrameCount&3==0` 命中（1551 mod 4 = 3）→ `data0D 60→5F`。**这是缺门链的直接后果**，不是计时器本身的算术错（rust 的 `&3` 判据与 ROM 一致）。

与 `$0454/$045E` 的关系：**不同链**。`$0454` 是 `Ens.isHit` bit5、`$045E` 是 `weaponAction`，由 `07:F2CA`（弹×敌判盒）写；`$040D` 是 `data0D`，由死亡/掉落与拾取计时写。f1301 修好后这两条不再出现，说明判盒链已一致；f1551 是下一条链。

## 5 可证伪预测

**主预测**：实现三道掉落门（quota、`MaxMissiles==0`、`L968B&1`）后，4000 帧腿的 `first_divergence` 应**离开 f1551** 且不再等于 `$040D`；本房这次死亡在 emu 侧是「无掉落」，rust 也应把 slot0 清 0，`$040D` 在 f1550–f1574 两侧都保持 `$60`。

**次预测**：修复后首个新分歧预计落在绘制/拾取面（OAM `$0200-$02FF` 或 `EnsExtra.animFrame`）——已知的同类绘制差在 **f2170 附近**的 `$020x` attr（emu `$21` / rust `$20`，`docs/findings/enemy-kill-drop-path.md:114` 记录 f2170/f2175/f2190 各差 44px、f2180 归零；成因注在 `crates/game-core/src/enemy/framework.rs:1034`）。若首分歧落在 f2170 附近的 `$02xx`，与本卡判定相容。

**否证判据**：

- 若修门后首分歧仍在 f1551 且仍是 `$040D` ⇒ 本卡根因判错（门链不是主因），退回重查 slot0 在 f1550 的实际 status。
- 若修门后首分歧跳到 f1550 的判等面（`$040D` 相等但 `$04xx` 其它字段差）⇒ 门实现时机不对（应在死亡同帧完成 RemoveEnemy，不能延后）。
- 若首分歧出现在 f1552–f1900 的 `$040D` 以外单字节 ⇒ 说明还有一条未识别的同帧写点，需按 §2 表逐写点插探针。

## 6 复跑（含 EnsExtra 观测量）

`tas_replay_diff.py` 的判等面限定 `$0100-$07FF`，看不到 `$6AF4`。需要 status/animFrame 时用 Mesen 场景单独取（不改 tools）：

```bash
# 1) 判等面复现（同上节命令）
# 2) EnsExtra 观测（临时场景，勿入库）：在生成场景里追加一行
#    watch_range 6AF4-6B04 extra0
python3 tools/scenario_run.py run /tmp/scn-x.txt
python3 - <<'PY'
import json
rows=[json.loads(l) for l in open('analysis/traces/tas-lordtom-metroid-100p-4000.frames.jsonl')]
for r in rows:
    if 1548<=r['f']<=1552:
        print(r['f'], r['w']['extra0'][:32])
PY
```

期望（本机实跑）：1548 `0608085D…`、1549 `0208085D…`、1550–1552 `00080837…`（status=0、animFrame=$37）。

## 7 边界与未覆盖

- 本卡只定界到「缺三道掉落门、本例栽在大能量资格」；三道门各自在哪些敌型/房间会先触发，未穷举。
- `MissilePickupQtyMax/Cur`、`EnergyPickupQtyMax/Cur`（`$93-$96`）rust 目前未建模（实测 emu f1550 `$96=1`、rust `$96=0`），属同一缺口的零页面，不在本卡判等面内。
- 未覆盖：Tourian 的 `Lx140` 分支（配额失败会重置配额并重试，且不 `RemoveEnemy`）——rust 里以 `DIVERGENCES 0x0DD5B` 记账，若将来进 Tourian 需单独验。
