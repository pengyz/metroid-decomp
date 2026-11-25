# 滚动期每帧工作量计费（V1 设计稿，Kraid 29/31）

> 状态：**设计稿（C 级拟合，未实现、未验证）**。V1 范围＝Kraid 29/31 两房归零；
> Tourian 6/10 与 Ridley 29/34/37/38 移 V2。本文只写设计与可证伪判据，
> 数值一律标「拟合」或「实测」，不把拟合当结论。

## 1. 问题

`room_sweep` 三区 7 房（Kraid 29/31、Tourian 6/10、Ridley 29/34/37/38）在滚动期与 emu 分叉
（15–18K px）。已定谳的分叉形态是**相机领先**（同帧 SamusX 两侧相同、ScrollX 差＝像素偏移，
Kraid 31 真偏移 6px），即 emu 在滚动期会「吞掉」部分帧、rust 帧帧满速。
证据见 [room-sweep §8.7](../findings/room-sweep.md)（:679 起）。

## 2. 已证实事实（A 级）

| # | 事实 | 证据 |
|---|---|---|
| F1 | rust 只在**载房窗**累计 `work_cycles`，滚动/对象更新期不累计 | `crates/game-core/src/world/camera/scroll.rs:46`（新载房清零）、`:71`（`dropped = ((LOAD_BASE_CYCLES + work_cycles)/FRAME_CYCLES).max(1)` → `iter_bogged`），全部累加点都在载房径：`crates/game-core/src/world/room_draw.rs:128/132/172/196/703/885` |
| F2 | 帧预算与载房基线的单一定义 | `crates/game-core/src/world/room_draw.rs:74` `LOAD_BASE_CYCLES = 28454`、`:75` `FRAME_CYCLES = 29780` |
| F3 | 周期模型现存只有**载房族** `model_*`，没有敌人 AI 的每帧周期模型 | `crates/game-core/src/frame/cycles.rs`（`model_fill_room_ram`/`model_draw_metatile_*`/`model_load_enemy` 等） |
| F4 | emu 滚动期丢帧节奏：a2 r31 = 1704/1709/1714/1719/1724（每 5 帧）；a4 r37 = 1701/1706/…；a3 r10 = 1694/1707/1712/…；a0 r05、a0 r21、a1 r03 = 0 | room-sweep §8.7 |
| F5 | 清空 6 个敌槽（`status=0`）后丢帧**全消失**、每帧 2px 无停顿 | `case-probe-lag-2-31-noen`（§8.7） |
| F6 | 滚动期活跃敌槽名册（`f60`/`f120` 采集，本次只读复算） | 见 §2.1 |

### 2.1 滚动期敌槽名册（复算口径）

口径：`$0400 + slot*16` 为敌对象；敌类型在 EnsExtra 镜像 `$6AF4 + slot*16 + 14`
（`crates/game-core/src/enemy/state.rs:67` `ENS_EXTRA::TYPE=14`、`:277` `ENEMY_STRIDE=16`、
`:127` `SLOT_COUNT=12`）；`ram.hex` 第二段正是 `$6AF4-$6BD3`（448 hex）。

| 区/房 | f60 活跃 | f120 活跃 | 类型 |
|---|---|---|---|
| a0 r21（不丢帧） | 5 | 5 | 7,6,7,5,5 |
| a1 r03（不丢帧） | 0 | 0 | — |
| a2 r29（丢帧） | 6 | 6 | 8,9,9,9,10,10 |
| a2 r31（丢帧） | 2 | 6 | 8,7 → 8,9,9,9,10,10（滚动中新生 4 敌） |
| a3 r06/r10（丢帧） | 3 | 3 | 1,0,1 |
| a4 r29（丢帧） | 4 | 4 | 12,12,12,12 |
| a4 r34/37/38（丢帧） | 4 | 4 | 2,3,3,3 / 2,2,3,3 / 2,2,3,3 |

复算命令（只读，产物不入库）：

```sh
python3 - <<'PY'
import glob
p = glob.glob('analysis/traces/_tmp_sweep_*_a2_r31_f01660_f60.ram.hex')[0]
t = open(p).read().strip().split('\n')[0]
ens = bytes.fromhex(t[4096:4096+448])
print([(i, ens[i*16+14], ens[i*16]) for i in range(12) if ens[i*16] != 0])
PY
```

### 2.2 由 F4–F6 得到的否证（单常量模型出局）

「每活跃槽 × 单一常量」（work = W0 + n·K）被三点同时否证：

- Kraid 29：n=6，每 5 帧丢 1；
- Tourian 10：n=**3**，同样每 5 帧丢 1；
- Brinstar 21：n=**5**、同样 2px/帧滚动，**零丢帧**。

W0+3K 与 W0+6K 都要跨过丢帧阈值 ⇒ K 必须很小 ⇒ n=5 的 Brinstar 无法不跨阈值；
反向按 Brinstar 定 K，则 n=3 的 Tourian 更不可能跨阈值。**故每帧成本必须依赖敌的类型/状态**。

### 2.3 逐帧停摆表（本次从磁盘复算，A 级）

口径：`frames.jsonl` 的 `w.ScrollX` 逐帧；滚动期正常 `+2`/帧，**增量 `0` 的帧即「停摆帧」**
（该帧的主循环迭代被推迟到下一个 vblank，摄像头不推进）。

| 区/房 | 滚动窗口 | 停摆帧（去掉载房窗 1601–1603 的爬升段后） | 间隔 |
|---|---|---|---|
| a2 r31 Kraid | 1604→1730 | 1706, 1711, 1716 | 5,5 |
| a2 r29 Kraid | 1604→1730 | 1659, 1664, 1669, 1674, 1679, 1692, 1701, 1706 | 5,5,5,5,13,9,5 |
| a3 r10 Tourian | 1604→1730 | 1696, 1709, 1714, 1719 | 13,5,5 |
| a3 r06 Tourian | 1604→1730 | 1678, 1691, 1696, 1701 | 13,5,5 |
| a4 r37 Ridley | 1604→1730 | 1703, 1708, 1713, 1718 | 5,5,5 |
| a1 r03 Norfair | 1604→1730 | **无** | — |
| a0 r21 Brinstar | 900→1020（该房自己的滚动窗） | **无**（2px/帧 × 118 帧，2→238） | — |

复算命令：

```sh
python3 - <<'PY'
import json, glob
p = glob.glob('analysis/traces/_tmp_sweep_*_a2_r31.frames.jsonl')[0]
w = {json.loads(l)['f']: json.loads(l)['w'].get('ScrollX') for l in open(p)}
print([(f, w[f]-w[f-1]) for f in range(1605, 1731) if w.get(f) is not None and w.get(f)-w.get(f-1) == 0])
PY
```

两条由表直接读出的结论：

1. **停摆周期 = 滚动 8px 一次**（2px/帧 × 4 帧 + 1 停摆 ⇒ 8px/5 帧），与「每 8px 流送一列名表」
   同周期；即**停摆就是列流送那一帧的工作量超帧预算**。
2. Brinstar 21 与 Norfair 03 在**同样 2px/帧滚动**下零停摆 ⇒ 差异不在滚动/列流送本身，
   与 §2.2 的敌槽对照一致（清空敌槽 ⇒ 停摆消失）。

### 2.3.1 停摆帧落在「列边界」上（A 级，决定性）

把停摆帧的 `ScrollX` 取出来看余数：

| 房 | 停摆帧的 ScrollX | `ScrollX % 8` |
|---|---|---|
| a2 r31 Kraid | 208, 216, 224 | **0, 0, 0** |
| a2 r29 Kraid | 112, 120, 128, 136, 144, 168, 184, 192 | **全 0** |
| a3 r10 Tourian | 186, 210, 218, 226 | **全 2**（该房滚动起点偏 2px，列边界=8k+2） |

⇒ **停摆只发生在「该帧要流送一列名表」的列边界帧上**，不是随机帧、也不是每帧。

但也不等于「列边界帧必停」：Kraid29 在 144→168 之间有 13 帧（边界 152、160 两帧都**没停**）。

⇒ 每帧工作量本身随敌槽名册/状态变化（敌生灭会让 `Σw` 变化），模型必须按**当帧实际名册**
计算，不能写成「每 8px 必停」的硬节拍。这条是 V1 标定的硬约束：
`144`→`168` 这两个边界帧的 `Σw` 必须落在预算内，而其余边界帧超预算。

### 2.3.2 `$2D` 帧计数器与停摆的相位关系（A 级，2026-11-23 实测）

探针 `analysis/_probe/lag-roster-31b.txt`（由 `tools/movie_ref/room_sweep.build_case(2,31,…)` 生成，
`preset=v1`、entry f1600、`watch 002d FC` + `watch_range 6af4-6bd3`）实跑 rc=0、1760 帧，
trace `analysis/traces/_tmp_sweep_1589184-1795444143_a2_r31.frames.jsonl`。逐帧复算命令：

```bash
python3 - <<'PY'
import json
seq={}
for l in open('analysis/traces/_tmp_sweep_1589184-1795444143_a2_r31.frames.jsonl'):
    r=json.loads(l); seq[r['f']]=r['w']
for f in range(1690,1736):
    a,b=seq[f],seq[f-1]
    ens=bytes.fromhex(a['EnsExtra'])
    ros=[ens[i*16+14] for i in range(12) if ens[i*16]!=0]
    print(f,'sx',a['ScrollX'],'dSX',a['ScrollX']-b['ScrollX'],
          'dFC',a['FC']-b['FC'],'n',len(ros),ros)
PY
```

结论（四条，均可从上面命令复现；写入计数用同生成器 + `write_hooks nt` 的另一次实跑，
trace 后缀 `.ntwrites.jsonl`）：

1. **列流送的 PPU 突发发生在「`ScrollX` 不推进」的那一帧**：r29 全窗每个 `dSX=0` 帧都恰好
   30 次 `$2007` 写（30 个不同地址 = 一列 30 行），非 `dSX=0` 帧 0 次写。
2. **`FC` 滞留发生在列边界帧本身**（即流送帧的前一帧）：该帧 `ScrollX` 刚跨到 `8k`、
   `FC` 增量为 0；下一帧 `ScrollX` 冻结、`FC` 恢复 +1、并出现列流写的 30 次突发。
   逐帧（r29）：`1658 sx=112 dSX=2 dFC=0 无写` → `1659 sx=112 dSX=0 dFC=1 30 写`。
3. **两个可观测量都要在 rust 侧复现**：① 边界帧 `FC` 不递增；② 其后一帧 `ScrollX` 冻结
   且发生一列 30 次 NT 写。`iter_bogged(1)` 的既有语义与第 ② 条一致（吞掉下一迭代 ⇒ 不推进）。
4. **单次列流的写入次数是常量（30），与列内容无关** ⇒ 列与列之间若成本不同，差异只能落在
   「取数/查表」那段代码，不在 PPU 写次数上（见 §2.3.3 的反例与排除）。

> 推断（B 级，未定谳）：贵列之所以「多花一帧」，可能是流送的取数段跨过了 vblank 边界、
> 把 30 次写挤到下一帧（所以写入被记在 `dSX=0` 帧上）。可证伪判据：若给列流送例程挂 PC 级
> 周期计数，贵列的取数段周期应显著大于便宜列、而写次数相同。

### 2.3.3 Kraid29：同名册下仍有「不超预算」的列（A 级，推翻单常量列成本）

探针 `analysis/_probe/lag-roster-29.txt`（同生成器，`watch 002d FC` + `watch_range 6af4-6bd3`），
trace `analysis/traces/_tmp_sweep_1629615-1795445195_a2_r29.frames.jsonl`。

- 从 f1676 到 f1740 名册**完全不变**（6 只，slot0–5 类型 `[8,9,9,9,10,10]`），但列边界帧的
  `dFC` 一半为 0、一半为 1：
  - 超预算（`dFC=0`）：`ScrollX = 112,120,128,136,144,168,184,192,200,208,216,224,232,240`
  - 未超（`dFC=1`）：**`152,160,176,248`**
- 复算（一行，可直接跑）：

  ```bash
  python3 -c "import json;seq={json.loads(l)['f']:json.loads(l)['w'] for l in open('analysis/traces/_tmp_sweep_1629615-1795445195_a2_r29.frames.jsonl')};print([(f,seq[f]['ScrollX'],seq[f]['FC']-seq[f-1]['FC']) for f in sorted(seq) if f>1606 and seq[f]['ScrollX']%8==0 and seq[f]['ScrollX']>seq[f-1]['ScrollX']])"
  ```

⇒ **`BASE + Σw[type] + 常量 COLUMN_COST` 在这一段必然给出统一结论，与实测冲突**；
这否掉的是「列成本为常量」这一条，不否掉「按名册加权」本身（r31 的 5 敌边界不超、
6 敌边界超，说明名册确实参与）。

已排除的替代解释（都基于同一批 capture/trace，不是推理）：

| 候选解释 | 实测 | 结论 |
|---|---|---|
| 空列省写入 | capture `f01720_f120.ppu.hex` 两页逐列非零 tile 数 = 26–30（全密） | 否 |
| 名表页切换 | 四条便宜列 `sx/8 = 19,20,22,31`，只有 31 在页尾 | 否 |
| 写入次数随内容变化 | 每个列流帧都恰好 30 次 `$2007` 写 | 否 |
| 名册/敌数变化 | f1676–f1740 名册不变 | 否（本段） |

⇒ V1 若只做「常量列成本 + 类型权重」，**在 Kraid29 上复现不出 `152/160/176/248` 这四列不停**；
要么补「按列/按取数段」的成本模型（需 ROM 例程级证据，V2），要么按 §5 交否定性结论。
> 【2026-11-23 更正】上句「按列/按取数段」的候选已被 §2.3.4 否掉：便宜列与贵列的列内容同构
> （都是 30 写 / 30 非零 / 同段值域），差别在**该帧有没有余量**，不在列本身。缺口定性因此改为
> 「敌人运行状态级成本」，V2 方向随之改变（不再是列成本分段函数）。

### 2.3.4 便宜列与贵列的「列内容」对照：列成本与列内容无关（A 级，2026-11-23）

按 lead 的时间盒要求（只查「为什么这 4 列便宜」，不再扩房、不再长跑），把同一 trace 的
`write_hooks nt`（30 字节列内容）逐列对照。四个便宜列与十个贵列的**列内容同构**：

| 列 | 分类 | 写入数 | 非零 tile | 取值种数 | 值域 |
|---|---|---|---|---|---|
| 96 | 便宜（同帧写完） | 30 | 30 | 6 | 8–255 |
| 104 | 便宜 | 30 | 30 | 6 | 8–255 |
| 112 | 贵 | 30 | 30 | 6 | 8–255 |
| 152 | 便宜 | 30 | 30 | 6 | 8–255 |
| 160 | 便宜 | 30 | 30 | 6 | 8–255 |
| 168 | 贵 | 30 | 30 | 6 | 8–255 |
| 176 | 便宜 | 30 | 30 | 9 | 8–255 |
| 184 | 贵 | 30 | 30 | 9 | 8–255 |

- 全部列都是 **30 次写、30 个不同地址、30 个非零 tile**；取值种数 6–11、值域同段。
  `sx=96`（便宜）与 `sx=112`（贵）内容画像完全一致，结论相反。
  ⇒ **「列内容/图案复杂度决定列成本」被否**；`UpdateNameTable`（`07:E590`）对每列固定循环 30 次
  `WritePPUByte`，构建成本本身也与内容无关。
- 真正区分便宜/贵的是**相位**：便宜列的 30 字节在**边界帧内**灌完（该帧 `FC` 前进），
  贵列的 30 字节落到**下一帧**（边界帧 `FC` 滞留）。逐帧数据：

  ```bash
  # 便宜（同帧写）：96,104,152,160,176,248
  # 贵  （次帧写）：112,120,128,136,144,168,184,192,200,208,216,224,232,240
  python3 -c "import json,collections;by={json.loads(l)['f']:json.loads(l)['w'] for l in open('analysis/traces/_tmp_sweep_1629615-1795445195_a2_r29.frames.jsonl')};c=collections.Counter(json.loads(l)['f'] for l in open('analysis/traces/_tmp_sweep_1629615-1795445195_a2_r29.ntwrites.jsonl') if 'f' in json.loads(l));print([(f,by[f]['ScrollX'],c.get(f,0)) for f in sorted(by) if f>1650 and by[f]['ScrollX']%8==0 and by[f]['ScrollX']!=by[f-1]['ScrollX']])"
  ```

- ⇒ 可证伪结论：**跨阈值的是「这一帧还剩多少余量」，不是列成本**。
  贵列/便宜列的差别只能由该帧**其它工作**（敌人/对象按运行状态计费）解释；
  非列帧从不超预算、列帧 14/20 超 ⇒ 非列工作的逐帧波动幅度与列成本同量级（≈8K 周期，约 27% 帧预算）。
- 这一条同时说明：**单列常量模型在 Kraid29 上不可能做到 14/14**，无论常量怎么调；
  要 14/14 必须给每个敌人按**运行状态**计费（逐个敌人例程建周期模型），属 V2。

### 2.4 敌槽名册与精灵数都不是充分解释（本次实测）

同一批采集里数 OAM 活跃精灵（`$0200-$02FF`，y∈(0,0xEF)）：Brinstar 21 = 31/23、
Kraid 31 = 25/33、Tourian 10 = 34/43、Ridley 37 = 31/40 ⇒ 精灵数与停摆**不同向**
（零停摆的 Brinstar 精灵数高于部分停摆房）。故「精灵数」「活跃敌槽数」均不能单独解释，
剩下的是**按类型的每帧 AI 工作量**（§2.2），V1 按类型加权标定。

## 3. 余下的一条读数不明（不影响 V1 建模）

§8.7 记的 `dFC` 数字（a2 r31「160/164 (+6)」等）与 §2.3 的停摆表**量级对不上**：
130 帧滚动窗里若有 ~26 个停摆帧，rust 的 FC 应比 emu 多出同量级，而不是 6。
`dFC` 的口径（哪个采集点、哪条腿减哪条腿）在文档里没有写全，**V1 不以它为标定输入**。
（2026-11-23 补充：§2.3.2 已用探针把「FC 在停摆帧是否同值连续两帧」实测出结果——
**是，且滞留发生在列边界帧本身、可见停摆在其下一帧**；§8.7 的 +6 仍属口径未写的旧读数，
按 lead 口径不改旧数、以本文 §2.3.2 为引用点。）
V1 的标定目标就是 §2.3 的停摆帧表本身（逐帧、可复算），比 dFC 更直接。

## 4. 候选模型（C 级拟合）

```
work(iter) = BASE_FRAME + Σ_{active slot} w[enemy_type] + COLUMN_COST（仅该帧流送名表列时）
if work(iter) > FRAME_CYCLES:
    iter_bogged(1)        // 下一个主循环迭代被吞：摄像头不推进、本轮更新晚一个 vblank —— 与载房窗同一机器
```

与 §2.3 的自洽性：列流送每 8px 发生一次，该帧超预算 ⇒ 吞掉下一迭代 ⇒ 那 8px 花 5 帧
（4 帧推进 + 1 帧停摆），停摆后像素进度重置，下一次列流送又在 8px 之后 ⇒ 稳定 5 帧周期。
清空敌槽后 `Σ w = 0` ⇒ 列流送帧回到预算内 ⇒ 零停摆（F5），与实测一致。

- `BASE_FRAME`、`w[·]`、`COLUMN_COST` 全部为**拟合量**（C 级），不是 ROM 周期核算；
  `frame/cycles.rs` 现有 `model_*` 只覆盖载房族，敌人 AI 逐帧模型 V1 不做（V2 若要 A 级须补 model）。
- **候选初值已按 §2.3.2 的夹逼关系重算（不再是随手取的圆整值）**，`F = FRAME_CYCLES = 29780`：

  | 量 | 值 | 由哪条实测夹出 |
  |---|---|---|
  | `BASE_FRAME` | 18000 | 非列帧 6 敌也必须跑完：`BASE + W6 ≤ F` |
  | `COLUMN_COST` | 8200 | 列帧 6 敌超、列帧 5 敌不超：`F - W6 < BASE + C ≤ F - W5` |
  | `w[8] / w[9] / w[10]` | 300 / **800** / 600 | `W6 = 300+3·800+2·600 = 3900`；`W5 = 300+2·800+2·600 = 3100`；差 `= w[9] = 800` 正好跨过 `F - BASE - C = 3580` |
  | `w[5..7]` 其它 | 300 | Brinstar `[7,6,7,5,5]` 列帧必须不超：`18000+8200+1500 = 27700 ≤ 29780` |

  三条判据的余量（可被 Kraid29 的 8 个停摆帧检验）：
  列帧 6 敌 `= 30100 > 29780`（超 320）；列帧 5 敌 `= 29300 ≤ 29780`（余 480）；非列帧 6 敌 `= 21900`（余 7880）。
  **Tourian 的 0/1 类型 V1 不标定**（3 敌就丢帧 ⇒ 其单敌权重须 ≥3580，且 V1 若给 300 必然复现不出，
  这属于 V2 输入，V1 验收不含 Tourian）。
- V1 只标定 Kraid 用到的类型 **8/9/10**；其它类型给保底权重并在本文标注「未标定」，
  V2 补 Tourian 0/1、Ridley 2/3/12。
- **离线回放结果（2026-11-23，用已落盘 trace 逐帧回放本文模型：超 `F` 就吞一个迭代）**：
  - Kraid31：emu 停摆帧 `1706/1711/1716/1721/1726/1731`，模型 **6/6 全中**；
  - Kraid29：emu 停摆帧 14 个，模型 **8/14**，错位来自 §2.3.4 的「便宜列表」与由此产生的相位漂移。
  ⇒ 机制方向（超预算吞迭代）成立，**模型类不足**（缺敌人运行状态级成本），见 §2.3.4 / §7。
- **已知会失的地方（§2.3.3 + §2.3.4，2026-11-23）**：上表这组值在 Kraid29 的
  `ScrollX = 96/104/152/160/176/248` 六个便宜列边界上会把「实际没超」判成「超」；
  已实测它们的列内容与贵列同构，故**不能**靠加列成本项修好，只能靠敌人状态级计费。
  本组值定位为**粗拟合起点**，V1 通过条件按 §5 收窄到 Kraid31 + 不回退房。
- 单一定义：常量与 `LOAD_BASE_CYCLES`/`FRAME_CYCLES` 同址（`world/room_draw.rs:74-75`），
  不新增第二处帧预算常量。

## 5. 可证伪判据（写死）

> 【2026-11-23 口径更新（lead seq17 时间盒裁定）】bounded 取证已做完（§2.3.4）：便宜列与贵列的
> 列内容同构，**给不出「列成本随内容变化」的单一可证伪解释** ⇒ 按裁定第 3 条**交否定性结论**，
> V1 以「Kraid31 6/6 + Kraid29 8/14 + 缺口定性」收，不硬凑 14/14。下列第 1、2 条按此改写。

V1 标定完成后，下面五条**同时**成立才算 V1 通过：

1. **Kraid 31 逐帧 `ScrollX` 序列与 emu 对齐**（同 fixture、同起止帧；`room_sweep --areas 2 --rooms 31`
   的 `diff==0`）；**Kraid 29 只作记录**：期望 8/14（§2.3.4 的六个便宜列是已知缺口），
   差值必须与本文预测逐帧一致，出现「表外偏差」才算失败；
2. 停摆帧落在 §2.3 表的实测位置：31 = 1706/1711/1716/1721/1726/1731（`endFrame` 采集口径）。
   注：§8.7 的 FC 读数（31 = 1701/1706/…）与这张表差一帧，是「同一停摆被两种采样点各记一次」，
   落地以 `room_sweep` 像素 `diff==0` 裁决，不以帧号凑数；
3. **Brinstar 21 与 Norfair 03 不得新增丢帧**（类型权重下界的守门条件）；
4. `cargo test -p game-core` 用例数 ≥ 1003 且全绿；
5. Tourian 6/10 在 V1 权重下若仍丢帧但节奏不匹配 ⇒ **记录为 V2 输入，不算 V1 失败**（不扩面）。

推翻判据（让本模型下台的实测）：任一新房在**滚动 2px/帧、活跃敌槽名册为 Kraid 同族类型
8/9/10** 的条件下仍不丢帧，则「类型权重」解释不成立，需转「按 ROM 周期核算」路线（补 `model_*`）。

## 6. 实现落点（V1 写面，两个 commit 各自可回滚）

| 文件 | 改动 |
|---|---|
| `crates/game-core/src/world/camera/scroll.rs` | `stream_column_kick`/`stream_row_kick` 里置「本迭代流送了一列」flag；移除只在 `is_load` 分支里计费的旧形（`:71` 的 `dropped` 只服务载房窗，保持原样） |
| `crates/game-core/src/frame/tick.rs` | `game_engine`（`:661`）尾部调 `account_iteration_work(mem)`：迭代开始时清 flag，结束按当帧名册+列流算 `work`，超 `FRAME_CYCLES` 就 `iter_bogged(1)` |
| `crates/game-core/src/world/room_draw.rs` | 新增 `BASE_FRAME_CYCLES`、`COLUMN_STREAM_CYCLES`、`enemy_frame_cost(type)` 权重表（与 `:74-75` 的 `LOAD_BASE_CYCLES`/`FRAME_CYCLES` 同址，保持单一定义） |
| `docs/spec/scroll-work-accounting.md` | 本文（标定完成后补实测表与残差） |

名册读取口径（与 §2.1 同一 API，不新增第二处解析）：`Manager::set_page_index(slot*16)` +
`Manager::enemy_type()`（`crates/game-core/src/enemy/state.rs:725`，读 `EnsExtra(+14)`）。

不碰 `samus/`、`render/`（属 #6/#12 写面），不碰 `gates/`、`data/status/`（属 #15 写面）。

## 7. 未覆盖 / 已知缺口

- 敌人 AI 逐帧周期模型缺失（F3）⇒ 本模型是拟合，不是周期核算；C 级。
- 类型权重的**状态依赖**（同一类型攻击/受伤/冻结时成本不同）V1 不建模。
- **V1 已知缺口（决定性，2026-11-23，§2.3.3/§2.3.4 实测）**：Kraid29 在名册恒 6 敌
  `[8,9,9,9,10,10]` 的条件下，20 个列边界帧里 14 个超预算、6 个（`ScrollX=96,104,152,160,176,248`）
  不超；这 6 个便宜列的**列内容与贵列同构**（30 写 / 30 非零 / 同段值域，见 §2.3.4 表），
  差别只在「30 字节是否在同一帧内灌完」。⇒ 缺口性质＝**敌人/对象按运行状态的逐帧成本未建模**，
  不是列成本、也不是类型权重。**V2 输入**：逐个敌人例程建状态级周期模型（PC 热点取证），
  V1 不取证、不硬凑。
- 滚动列流（`UpdateNameTable` 每 8px 列）的固定构建成本并入 `COLUMN_COST`／`BASE_FRAME`；
  §2.3.4 已否掉「列成本随内容变化」，故不拆成按列分段函数。
- V1 交付口径（lead seq19 裁定）：Kraid31 逐帧 `ScrollX` 对齐 + Brinstar21/Norfair03 不新增停摆；
  Kraid29 与 Tourian/Ridley 五房均**不作为 V1 通过条件**，随本文件缺口记录转 V2。
- Tourian / Ridley 五房与 Brinstar 47 房的全量回归属 V2 验收。

### 7.1 状态级成本模型（V2 输入，2026-11-24；结论：被否证）

**目标**：把「每敌每帧成本」从类型权重细化到运行状态，解释 §2.3.3 的 14 超 / 6 不超。

**取证面（只读既有 trace，不启 emu、不改 `crates/`）**：`analysis/_probe/stall_cost_model.py`
（脚本不入库，随卡交付）。输入 `analysis/traces/kraid29-sweep.frames.jsonl`（稳定副本；
原为 2026-11-23 sweep 临时产物 `_tmp_sweep_1629615-1795445195_a2_r29.frames.jsonl`，
内容逐字节相同，sha256 `ea8eb397591e58cf335029ac59d52d12569ee2e1493b09a6a250786c2da1cf26`。
**重采需 emu，当前平台不可用**（Mesen `--testRunner` 报 `std::bad_cast`、GUI 秒退），
故以该文件为唯一数据源）。
判定口径：列边界帧 = `ScrollX % 8 == 0` 且比前一帧大；超预算 = 该帧 `FC` 增量 0（§2.3.2）。
`EnsExtra = $6AF4 + slot*16`，字段 0=status、3=animFrame、4=animDelay、6=animIndex、
13=jumpDsplcmnt、14=type、15=data1F（`crates/game-core/src/enemy/state.rs`）。

```sh
python3 analysis/_probe/stall_cost_model.py --area 2 --room 29
```

**Kraid29 边界帧实测**（窗口 1650–1740，20 帧；`ScrollX`→dFC）：

| ScrollX | 96 | 104 | 112 | 120 | 128 | 136 | 144 | 152 | 160 | 168 | 176 | 184 | 192 | 200 | 208 | 216 | 224 | 232 | 240 | 248 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dFC | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| 判定 | 便宜 | 便宜 | 超 | 超 | 超 | 超 | 超 | 便宜 | 便宜 | 超 | 便宜 | 超 | 超 | 超 | 超 | 超 | 超 | 超 | 超 | 便宜 |

便宜列 = `{96,104,152,160,176,248}`（6 个），超预算 = 其余 14 个 ⇒ 与 §2.3.3/§7 的旧表一致。

**否证 1：类型权重在 Kraid29 上不自洽**。名册 `[8,9,9,9,10,10]` 的 19 个边界帧里同时出现
14 个超、5 个便宜（96/104 属过渡名册 `[8,10]`）⇒ 同一名册、相反结论，任何 `w[type]` 都不可分。
（Kraid31 无此冲突：6 敌名册的 6 个边界帧全超，见下。）

**否证 2（本轮关键）：末态字段无法支撑状态级拆分**。

- 以 `(type,status)` 为状态：31 个边界帧只有 4 个唯一状态向量，其中 **2 个向量内部标签冲突**
  （19/31 行参与冲突）。例：`[(1650,96,便宜),(1658,112,超),(1663,120,超),…,(1683,152,便宜),(1687,160,便宜)]`。
- 以 `(type,status,animFrame,animIndex,data1F)` 为状态：7 个唯一向量，仍 **2 组冲突、11 行参与**。
- 逐字节核对：这些冲突行的 6 槽 × 16 B `EnsExtra` 快照在冲突组内**完全一致**
  （如 1663 与 1683 的 96 B 全等），只有 `animDelay` 之类计数器不同；而 `dFC` 相反。
  ⇒ 超预算与否由**帧内执行路径**决定，不由帧末状态决定；状态级线性模型在此数据面上被否证。

**否证 3：超预算不是「列边界必停」的硬节拍**。全窗 `dFC=0` 帧（不含入场 1601–1603）：
`1658,1663,1668,1673,1678,1691,1700,1705,1710,1715,1720,1725,1730,1735,1744`，
间隔 `5,5,5,5,13,9,5,5,5,5,5,5,5,9`；其中 **1744 不是列边界帧**（无列流送也滞留 FC）。
⇒ 存在一条 5 帧量级的周期工作量（便宜列只是「该周期没撞上列成本」的边界帧），
这些便宜列的列内容与贵列同构（§2.3.4），差别是相位而不是列或类型。

**守门判据的现状（如实记录，不硬凑）**：

- Kraid31 6/6：本脚本可复算 —— `python3 analysis/_probe/stall_cost_model.py --area 2 --room 31`
  （数据源 `analysis/traces/kraid31-sweep.frames.jsonl`，同为 2026-11-23 sweep 产物的稳定副本）
  → 6 敌名册的 6 个边界帧 `ScrollX=208,216,224,232,240,248` 全 dFC=0，无同名册冲突。
- Brinstar21 / Norfair03 零停摆：**现有 trace 不足以复算** —— 全仓仅 3 份 `frames.jsonl` 同时含
  `EnsExtra` 与 `FC`（r29/r31/lag-roster-31），a0_r21 与 a1_r03 的 trace 只有 `ScrollX` 等 8 个字段
  （无 `EnsExtra`、无 `FC`），`FC` 判定面缺席 ⇒ 该守门项本轮**未执行**，不写「一致」。

**V2 需要补的数据（具体到字段，不泛泛）**：

1. 每帧 `watch_range 0400-04FF`（敌对象本体：`data05`/`movementIndex`/`delay`/`data0A`/`health`）——
   本轮只有 `$6AF4` 的「附加面」，帧内 AI 状态在 `$0400` 面，未采。
2. 每帧敌例程 PC 级周期计数（`write_hooks cpu` 或 PC 采样），用来把 5 帧周期工作量按子例程归因。
3. 对照房（a0_r21、a1_r03）补采 `watch 002d` + `watch_range 6af4-6bd3`，否则守门项无法判定。

**与 V1 的差异**：V1 用 `BASE+Σw[type]+COL`（§8.1）；本节的否证说明「状态」若只取帧末字段，
在 Kraid29 上无解，V2 必须走帧内执行路径（PC/循环次数），而不是再加一层末态特征。
标 **C 级拟合**：以上表格是点数，机制归因（5 帧周期来自哪个例程）未取证。

## 8. V1 实现实测（2026-11-23，拆骨）

### 8.1 落地形态

- `Manager` 新增 `frame_streamed: bool`（`crates/game-core/src/state.rs`）：本拍是否做过
  流送列/行写；`stream_column_kick` / `stream_row_kick`（`crates/game-core/src/world/camera/scroll.rs`）
  首行置位；`game_engine`（`crates/game-core/src/frame/tick.rs`）每拍起清。
- `account_iteration_work(mem)`（同文件）：`room_load_phase != 0` 直接返回（载房窗仍由
  `check_update_name_table` 的 `is_load` 径自算 `dropped`）；否则
  `work = BASE_FRAME_CYCLES(+COLUMN_STREAM_CYCLES if frame_streamed) + Σ enemy_frame_cost(type)`，
  名册遍历 `0..SLOT_COUNT`，`status_raw()!=0` 为活跃；`work > FRAME_CYCLES` ⇒ `iter_bogged(1)`。
- 常量单点定义在 `crates/game-core/src/world/room_draw.rs`：`BASE_FRAME_CYCLES=18000`、
  `COLUMN_STREAM_CYCLES=8200`，`enemy_frame_cost(t)`：`8→300 / 9→800 / 10→600`，其它类型
  **未标定**给 300（保底）。

### 8.2 Kraid31 逐帧 `ScrollX` 对齐（A 级）

复跑（rust 腿 1.9 s；emu 侧复用既有 `frames.jsonl`）：

```
python3 - <<'PY'
import json
emu={}
for l in open('analysis/traces/_tmp_sweep_1589184-1795444143_a2_r31.frames.jsonl'):
    l=l.strip()
    if not l or l=='{': continue
    try: d=json.loads(l)
    except: continue
    emu[d['f']]=d['w']['ScrollX']
rs={}
for l in open('analysis/scratch-sprite-residual/kraid31-post21/kraid31-post21.state.jsonl'):
    d=json.loads(l); rs[d['f']]=bytes.fromhex(d['w']['zp'])[0xFD]
rng=range(1600,1752)
print('缺行', [f for f in rng if f not in rs])
print('不一致', [(f,rs[f],emu[f]) for f in rng if f in rs and rs[f]!=emu.get(f)])
PY
```

实测输出：

```
缺行 [1601, 1602, 1705, 1710, 1715, 1720, 1725, 1730, 1735]
不一致 []
```

- **共有帧 `ScrollX` 不一致 = 0**（143 帧全部逐字节相等）。
- 「缺行」= 该帧 `iter_bogged(1)` 吃掉整拍、`--state-out` 不落行；把缺行帧 +1 与 emu 的
  停摆帧（`ScrollX` 增量为 0 且非 0 值）比较：`{1705,1710,1715,1720,1725,1730}+1 = {1706,1711,1716,1721,1726,1731}`
  与 §2.3 停摆表逐项相等（载房窗 `1601,1602`+1=`1603` 亦对上）。
- **已知偏差（不影响 `ScrollX` 读数，如实登记）**：
  1. 存活的 bog 行比 emu 停摆帧**早一帧**（rust 1705 ↔ emu 1706）。原因是 `iter_bogged` 在
     本拍末尾推门、下一拍整拍跳过，而 emu 的「增量 0」标在到达帧上；两侧可观测的 `ScrollX`
     序列因此完全一致，但「哪一帧被标成停摆」差 1。
  2. 尾部多一个 bog 行 `1735`（+1=`1736`）：该帧 `ScrollX` 由 254 回绕 0（滚动结束），
     emu 侧同样不前进，但「增量 0」判据在回绕帧上不可分辨，属**不可见的等价 bog**。

### 8.3 Brinstar21 / Norfair03 不新增停摆（A 级名册 + B 级上界）

emu 名册（`EnsExtra` 逐槽，`analysis/traces/*_a0_r21` / `*_a1_r03` 的 `ram.hex` 段二）。
**下表名册取「窗口内各采集帧的最大名册」，不是单帧**（2026-11-24 对影复核勘误：
原 `Norfair03 [5,5] → 26800` 只是 `_tmp_sweep_2220997…_f00960_f60` 那一帧的读数）：

| 房 | 窗口内名册上界（类型） | 列帧 work 上界 | ≤ `FRAME_CYCLES`(29780) |
|---|---|---|---|
| Brinstar21 | `[7,6,7,5,5]` | 18000+8200+1500 = 27700 | 是 |
| Norfair03 | `[5,5,3,3,5,5]` | 18000+8200+1800 = 28000 | 是 |

- 两房 emu 轨迹在滚动段（`ScrollX≠0`）内**零停摆**；V1 的 bog 条件是 `work > 29780`，
  上述上界严格小于 ⇒ **结构上不可能新增 bog**（不是"这次没触发"，是条件不可达）。
- 局限：这两房的名册来自上一轮采集，若 V2 改权重需重采名册复算。

### 8.4 Kraid29（不作 V1 通过条件）

`[8,9,9,9,10,10]` 恒 6 敌下 20 个列边界帧 14 超 / 6 不超（`ScrollX=96,104,152,160,176,248`），
即 §2.3.4 的「便宜列」缺口；V1 会把这 6 帧误判为超预算。按裁定：**如实登记为 V2 输入**，
不调权重硬凑、不改判据。

## 9 A 方案落地：载房窗精灵页续写（#24）

### 9.1 机制（与 ROM 的对应）

ROM 的 NMI 把对象逐槽写进 `$0200` OAM 影子页；载房请求在绘制途中抬起时 NMI 提前收口，
**未重写的槽保留上一帧内容**（PPU DMA 每帧整页拷贝 256B，不会自动清）。Rust 侧原先每帧
`update_world` 开头 `set_sprite_page_pos(0)`、窗内整帧跳过且尾清 `$F4` ⇒ 窗后首帧从 0 重画，
窗内则把「上次写过的槽」清成空位，于是**精灵消失**（f1100 266px / c676 303px 的根因）。

A 方案按 ROM 语义补三件事：
- `state.rs` 新增 `draw_interrupted` / `sprite_resume_pending` / `in_update_world` 三布尔；
- `world/camera/scroll.rs` 的 `is_new_load_request` 分支：若本次载房是在 `update_world` 内抬起
  （`in_update_world`），置 `draw_interrupted = sprite_resume_pending = true`；
- `world/update.rs`：`update_samus` 之后若 `draw_interrupted` 则**跳过其余件序与尾清**并返回（游标保留）；
  `render/oam.rs::obj_draw_frame` 在打断态下拒画；窗后第一个迭代**不重置** `sprite_page_pos`，
  从断点续写、并把续写游标之后的槽照常尾清。

### 9.2 实测（172 采集点，逐像素对 `shots/` 的 emu 参考图）

复跑（无需 emu：只用 `shots/` 里的参考图）：

```
python3 - <<'PY'
import sys; sys.path.insert(0,'tools')
import l4_diff_run as L, scenario_run as sr
from pathlib import Path
from PIL import Image; import numpy as np
OUT=Path('/tmp/diag21'); OUT.mkdir(exist_ok=True); L.SHOTS_RUST=OUT
for sp in sorted((L.ROOT/'cases').glob('*.txt')):
    case=sp.stem; plan=sr.parse_scenario(sp); frames=int(plan['frames'])
    joy,joy2=sr.expand_pads(plan); caps=sorted((int(c['frame']),c['tag']) for c in plan.get('captures',[]))
    L.write_schedule(case,frames,joy,caps,OUT/f'{case}.sched',fixtures=plan.get('fixtures'),joy2=joy2)
    L.run_rust(case,OUT/f'{case}.sched',OUT)
d=lambda a,b: int((np.array(Image.open(a).convert('RGB'))!=np.array(Image.open(b).convert('RGB'))).any(axis=2).sum())
for e in sorted(Path('shots').glob('case-*.png')):
    r=OUT/e.name
    if r.exists():
        n=d(e,r)
        if n: print(e.name, n)
PY
```

结果：**172 采集中 168 逐字节相同、4 处非零**，与基线同为 4 格但成分变了：

| 格 | #24 落地后 | 基线 | 说明 |
|---|---|---|---|
| `case-scrollseam-probe` f1100（整案 14/14） | **0** | 266 | 本卡目标达标 |
| `case-mechanism-input-conflict` c676 | **0** | 303 | 本卡目标达标 |
| 同案 c674 | 84 | 84 | 不恶化；与 c678 同像素带 (100,81)-(132,88) |
| 同案 c678 | **32** | 0 | 新增，见 9.3 定性 |
| `case-mechanism-door-transition` f486 | **369** | 0 | 新增，见 9.3 定性 |
| `case-password-justin` f1191 | 2063 | 2063 | 既有分歧（`$FE` 熄屏早一 tick），与 A 无关 |

21 个用例里 **18 个逐字节相同**（input-conflict / door-transition / password-justin 三案有非零格）。

### 9.3 两处新增差的定性（A 级字段证据）

- **door f486**：`rust --trace oam:486-486` 33 个可见槽 vs emu `_f00486_load-chain.ram.hex`
  的 `$0200` 页 25 个；rust 槽 0-7 是门帧精灵（`y31/39 tile CC/CD/DC/DD attr62`）、12-15 又是同一组，
  即**旧页残留 + 续写重复**；emu 槽 0-7 直接是 `CA/CB/DA/DB ×2`。同拍 `$5A`：rust `0xFF`、emu `0xF4`
  ⇒ Rust 的载房游标已到尾段而 ROM 还在 `$F4`，**该帧 ROM 的载房请求并未落在绘制途中**。
  归属：本卡把「保留旧页」打开后，把这个既有的载房相位差（`$5A` 早到尾段，属 #21 的 `$5A F0/F1` 尾段面）
  显影成了 369px；基线 0px 是尾清掩盖。**不是 A 方案引入的新根因**，但**本卡未修**。
- **input-conflict c678**：32px 落在 (100,81)-(132,88)，与 c674 的 84px 同一像素带 ⇒ 同一对象残差；
  本卡不单独修（c674 基线即 84px，属既有残差面）。

### 9.4 已试并否决的变体（证据在数上）

| 变体 | c676 | f1100 | f486 | c678 | 结论 |
|---|---|---|---|---|---|
| v13 原形 | 0 | 0 | 369 | 32 | 采用 |
| 中断帧仍做尾清 | 478 | 342 | — | — | 否决：保留旧页就是硬件语义，清不得 |
| `in_update_world && frame_streamed` 才打断 | 0 | 0 | 369 | 32 | 否决：门链载房那拍也踢了列，判据不区分 |
| `room_load_phase != 0` 时不打断 | 0 | 0 | 369 | 32 | 否决：门链载房时相位已是 0 |
| 打断点移到 `update_world` 末尾 | 209 | 191 | 250 | 48 | 否决：四格一起退化 |

### 9.5 未覆盖

- f486 的正确修法是让「门/电梯转场在帧界触发的载房」不打断对象环（卡面待修 #2），本卡只做了定性；
- 打断点仍按「阶段」而非「敌环槽游标」定位（卡面待修 #1），c678 32px 的成因未拆到单一字段。

### 9.6 首轮复核退回后的修正（2026-11-24）

**已修**：`world/update.rs` 正常路径缺 `in_update_world` 复位（首帧之后恒 true ⇒ 帧界门链载房
（`tick.rs` 的 `scroll_door` → `scroll_left/right` → `check_update_name_table`）被误判成「对象环
中途打断」）。已在正常 return 前补 `mem.in_update_world = false;`。

**复测（修正后）**：door-transition 案 f486 仍是 **369px**（`--trace oam:486` 33 可见槽）。
临时插桩（已删）显示整个 door 案只触发一次中断：`rn=0x17 pos=32`（页游标在 slot8 被切断），
即除陈旧旗之外还有一次**真实发生在 `update_world` 内的载房**；ROM 在 f486 这拍是整页重写
（emu 侧 25 槽）。⇒ 卡面「待修 #1（打断点按敌环槽游标而非阶段）」仍是未修项。

**判定面受限（未覆盖，不作结论）**：
- 本机 Mesen 现启动即 `std::bad_cast` 中止，`--fresh-emu` 全部返回「缺模拟器截图」；
- `shots/` 的参考图被多代际运行覆盖（door f486 的 png/trace mtime 为 11-24 00:16，其余同批为
  11-18 22:05；scrollseam 的 case 图被改名为 `probe-scrollseam-win_*`），离线逐像素矩阵已不可信
  （同一代码对 `shots/` 复算 f1100 得 17036px）。
- 因此本卡的「21 案逐案表 + 干净树 HEAD」「c678 按面拆分」两项**未完成**，等 Mesen 恢复后补。

### 9.7 door 案不止 f486：560/600 两格大残差（对影 fresh-emu 实测，2026-11-24）

对影 seq7 在 `/tmp/rev24`（未含 9.6 的旗复位）用同一次 fresh-emu 采到 door 案六格：
`470 approach 0 / 481 door-trigger 0 / 486 load-chain 369 / 500 transition 0 / 560 in-room 22344 / 600 end 24359`。
**560/600 的 22k/24k 是整屏级差异**，本卡首轮提交只登记了 f486，漏了这两格——已在 §9 补齐登记。

同一探针在本仓三案的置旗点读数（临时插桩，已删）：

| 案 | rn | pos | $0300..$0303 | $5A | $FD |
|---|---|---|---|---|---|
| door-transition | 0x17 | 32 | 01,0F,04,03 | 17 | FF |
| input-conflict | 0x17 | 32 | 01,0F,04,03 | 17 | FF |
| scrollseam-probe | 0x14 | 32 | 01,0F,04,03 | 14 | 01 |

三案**都在「进房首次载房」这一拍触发同一形状的中断**（`pos=32`、状态字节相同）。这说明 v13 的
「载房那拍跳过 `update_samus` 之后的全部 `uw_*` 件序」是一个**过粗的模型**：ROM 的 SetupRoom 是
`jsr` 返回后继续走余下件序的，Rust 侧整段跳过才会让门链案在 560/600 把门/瓦爆/碰撞/HUD 的推进
整帧丢掉，进房后状态分叉。⇒ 卡面「待修 #1（按敌环槽游标定位打断点）」不只是精度问题，
**是 A 方案能否成立的前提**；按当前形态不应合入。

结论（本卡内）：A 方案的两格收益（f1100、c676）已由对影 fresh-emu 复现，但代价是 door 案
560/600 两格整屏级回归 + f486 未修，且正确修法（槽游标 + 门链径排除）需要 emu 判定面，
而本机 Mesen 已不可用（§9.6）。建议：**A 方案暂缓合入**，已修的 `in_update_world` 复位单独保留
（它是与 A 无关的正确性修复），A 的重做另开卡。

### 9.8 ROM 侧对照：SetupRoom 之后「只跳过名字表流送」，不是跳过整帧（2026-11-24）

m1disasm（`refs/m1disasm/SRC/prg7_engine.asm`，A 级）：

```
CheckUpdateNameTable: ; 07:E54A
        jsr SetupRoom            ; 载房（07:EA2B）
        ldx RoomNumber           ; 新房载入后 RoomNumber 被清为 $FF
        inx
        bne RTS_X173             ; 不是 $FF ⇒ 直接 rts，跳过本帧列/行流送
        lda ScrollDir
        ...
        jmp CheckUpdateNameTableHorizontal / Vertical
```

调用链是 `UpdSamus → ScrollLeft/Right/Up/Down → CheckUpdateNameTable → SetupRoom`（`07:E54A` 处 `jsr`），
`rts` 只回到 `UpdSamus`，**对象更新循环与后面的 `uw_*` 件序照常继续**。所以 v13「载房那拍把
`update_samus` 之后的件序整段跳过」在 ROM 里没有对应物——ROM 跳过的仅是 **本帧的名字表流送**。

这解释了 §9.7 的实测：f1100/c676 之所以被修好，是因为精灵页在本帧不再继续写（等价于「不流送」），
而 door 案 560/600 的 22k/24k 是因为我们把门/瓦爆/碰撞/HUD 的推进也一起丢了。
⇒ A 的重做应改为：**只在载房拍抑制「精灵页续写 + 名字表流送」**，其余件序照常跑；
验收必须含 door 六格 0 与 f1100/c676 不回退。

### 9.9 更正：door 560/600 的两格差**不是** A 方案造成的（2026-11-24，实测）

对影 seq7 的 fresh-emu 读数（560=22344、600=24359）我复核后确认数字能复现，但**因果归属要更正**。
以她 `/tmp/rev24/shots/` 里的 emu 参考图为准，逐层剥：

| 树状态 | f486 | f560 | f600 |
|---|---|---|---|
| 干净 HEAD（`git stash` 掉我全部改动，只留他人改动） | **0** | 22344 | 24359 |
| HEAD + #24 A 方案（含 `in_update_world` 陈旧旗） | **369** | 22344 | 24359 |
| HEAD + A 方案 + 旗复位（上一轮提交态） | **369** | 22344 | 24359 |
| HEAD + 旗复位、A 撤回（本轮最终态） | **0** | 22344 | 24359 |

⇒ 结论三条：
1. **A 方案的真实回归只有 f486（0→369）**，撤回 A 后回到 0；
2. **560/600 的 22k/24k 在干净 HEAD 上就已存在**，与 A、与 #21 的 V1 计费都无关
   （另跑过一版把 `account_iteration_work` 注释掉的构建，560/600 一字不差）；
3. 因此 §9.7 里「560/600 是 A 整段跳过件序的直接后果」这句**作废**——那是把两件独立的事
   归成了一因。它们应在门链案自己的卡上查（进邻房后的状态分叉，疑与 `shots/` 参考图代际有关）。

另注：`shots/` 在 2026-11-24 00:23 被整批改名成
`mechanism-<case>-<frames>-<sha>_f*.png` 形态，`case-mechanism-input-conflict` 的 c673/c674 参考图
在新命名下已找不到对应项，旧的离线矩阵（172 点 168 同）**不可再直接引用**，需重新采参考。

#### 9.9.1 对影独立回验：door 六格在正确代际参考图下全 0（2026-11-24）

对影用「干净副本（`git archive HEAD`）+ 当前树副本」，把 `shots/` 里 `aa12a65e` 代际的
`mechanism-case-mechanism-door-transition-600-aa12a65e_f*.png` 按 l4 期望名建 symlink 后跑
`--skip-emu`：**两棵树都 6/6 全 0**（470/481/486/500/560/600）。⇒ 9.9 表里的 560=22344/600=24359
是「非 aa12a65e 代际」的参考图造成的读数，**不是代码分歧**；A 撤下后 f486 也回到 0。
本节的「应单开卡查」相应收窄为：若日后仍要追 560/600，必须先固定参考图代际，否则读数是假的。

### 9.10 v14b：模型修正（不早退 + 分拍打断）实测（2026-11-24，Task #27）

#### 9.10.1 与 v13 的差别

v13 在 `update_samus` 之后 **early return**，把 AreaRoutine / 门链 `uw_*` / 尾清全跳过——这不是 ROM 行为：
ROM `CheckUpdateNameTable ; 07:E54A → jsr SetupRoom ; 07:EA2B` 的调用点在 `UpdateSamus` 内部，
`SetupRoom` 到 `RoomFinished ; 07:EA26` 才 rts，返回后**余下件序照跑**（§9.8）。
v14b 因此只改三件事：

1. `update_world` **不早退**（除非本拍被打断且不是「本拍才抬起」，见 2/3）；
2. `SpritePagePos←0` 每帧照旧（冻结窗那两拍根本不执行 `update_world`，断点游标天然停留）；
3. 打断处理**分两拍**：载房请求在 `in_update_world && in_samus_update` 时置 `draw_interrupted`；
   - **本拍才抬起**（`load_started_this_frame`）⇒ 件序全部照跑（门链要靠它们推进，v13 一刀切才把 f486 打成 369）；
   - **窗后续写拍**（进入本拍时 phase≠0）⇒ `obj_draw_frame` 不写页 + 不尾清 + 提前返回。

字段落点：`crates/game-core/src/state.rs` 的 `Manager` 加 `draw_interrupted` / `in_samus_update` /
`load_started_this_frame` 三个 bool（断点状态的唯一落点；`world/`+`frame/` 写面外的最小必要越界，
与 #21 的 `frame_streamed` 同一处理方式）。

#### 9.10.2 变体矩阵（同一 HEAD，`l4_diff_run.py run <case>`，fresh emu 自采参考图）

| 变体 | door 470/481/486/500/560/600 | scrollseam f1100 | input c674 | c676 | c678 |
|---|---|---|---|---|---|
| 干净 HEAD（基线） | 0/0/**0**/0/0/0 | 266 | 84 | 303 | 0 |
| v13 原型（早退 + 全局守卫） | 0/0/**369**/0/0/0 | **0** | 84 | **0** | 32 |
| v14a（不早退 + 跳尾清，无守卫） | 0/0/**0**/0/0/0 | 266 | 84 | 303 | 0 |
| v14b（不早退 + 全局守卫） | 0/0/**126**/0/0/0 | 191 | 84 | 209 | 16 |
| **v14b（分拍：只有窗后续写拍才打断）** | **6/6 全 0** | **191** | 84 | **237** | **0** |

⇒ 三条实测结论：
1. **door 560/600 的 22k/24k 与早退无关**（v14a 去掉早退后仍 0，与 §9.9.1 的代际结论一致）；
2. **door 486 的回归来自「在载房起拍就停笔」**：分拍后回到 0；
3. **f1100 的 266 → 191、c676 的 303 → 237** 来自「窗后续写拍停笔」，但**没有归零**——说明这两格的剩余差
   不是「多画了 Samus 之后的件」，而是**精灵页在窗内的字节内容**本身还差（下一步要按槽比 `$0200`，
   而不是再调旗的时序）。

#### 9.10.3 复跑

```sh
cargo build -p game-core --bin rust_screenshot
python3 tools/l4_diff_run.py run case-mechanism-door-transition
python3 tools/l4_diff_run.py run case-mechanism-scrollseam-probe
python3 tools/l4_diff_run.py run case-mechanism-input-conflict
```

#### 9.10.4 未达成（卡面硬指标）

- `f1100 = 0`、`c676 = 0` **未达成**（191 / 237）。已排除的解释：早退、尾清、门链误伤、起拍停笔。
  下一步观测面：窗内两拍（`rl=1`）rust 与 emu 的 `$0200-$02FF` 逐槽对照 + emu 该拍的精灵页写计数
  （`oamwrites` 采点已在 `probe-scrollseam-win` 产物里有样本），确定是「窗内旧字节保留范围」还是
  「窗后从 0 重画」哪一边错。

### 9.11 v14c：窗内「空推进游标」——f1100 与 c676 归零（2026-11-24）

#### 9.11.1 模型修正（A 级证据）

§9.10 的「停笔」模型只压到 `f1100=191`/`c676=237`。逐槽比 `$0200`（emu
`analysis/traces/case-scrollseam-probe_f01100_scroll-r180.ram.hex` vs rust `--state-out` f1100 的
`iram[0x200..0x300]`）给出真正的差别：

| 槽 | emu（显示面） | rust（v14b） |
|---|---|---|
| 5..11 | 窗前旧字节 `(176,0,96,143)` 等 | 本帧 7 个敌精灵 `(33,163,1,48)` 等 |
| 12..18 | 同一批 7 个敌精灵 | 后续对象 |

（C 级解释，未定谳，2026-11-24 两次更正）⇒ 初读把槽 5..11 当成「窗前旧字节」，织码复核指出
**槽号读串、attr 十进制/十六进制看花**：`(176,0,96,143)` 就是 y=176/tile=0x00/attr=0x60/x=143 的新姿组本身，
两份 ram.hex 的 `$0200-$02FF` 整页 sha 相同（`699cbd3e…`）。因此本节标题「窗内不写」**不是** ROM 语义，
只是一个在像素判据上有效的实现形；真实写次数由 `probe-scrollseam-win.oamwrites.jsonl` 给出：
f1098 写 0 次、f1099 写 128 次（槽 5–63，发生在帧内较晚处）。机制归属见 §9.11.5.3，仍需另开卡定谳。
本节的**实测结果不受影响**（下节三案像素判据）。

#### 9.11.2 实现（`crates/game-core/src/render/oam.rs`，单文件）

- 删掉 `obj_draw_frame` 头的早退守卫（窗内对象继续走 `ObjDrawFrame` 全径：动画帧/径/可见性判定都照跑）。
- `draw_sprite_stream` 增加 `dry = mem.draw_interrupted && !mem.load_started_this_frame`：
  仍按帧流算每一枚精灵的 `y/tile/attr/x` 与槽位进位，但 `if !dry` 才写 `$0200+`；循环尾照旧
  `set_sprite_page_pos(x_sprite)` ⇒ 等价于「消耗页槽、不写字节」。
- 可见性门、`ObjectCntrl` 收尾、`0xFC/0xFD/0xFE/$FF` 分支都不变（不新增语义分叉）。

#### 9.11.3 实测（fresh emu 自采参考图）

| 用例 | 采集点 | 结果 |
|---|---|---|
| `case-mechanism-door-transition` | 6 | **6/6 全 0**（470/481/486/500/560/600） |
| `case-mechanism-input-conflict` | 11 | 10 一致；c673=0、c674=**84**、c676=**0**、c678=**0** |
| `case-scrollseam-probe` | 14 | **14/14 全 0**（含 f1100） |

`cargo test -p game-core` 合计 **1033 passed / 0 failed**；`gates/cli.py check` **PASS=24 FAIL=0 SKIP=3 TOLERATED=1**。

#### 9.11.4 未达成/边界（含机制标注）

- `c674=84` 保持原样——它属 `$FE` 早一 tick 写（密码提交转场链）的既有分歧，不归本模型。
- `$5A` 的 `F0/F1` 相位差（§9.10.1）未单独修：本轮实测在 v14c 下 f1100 已归零，相位差不单独表现为像素差；
  若后续采到新相位点再按「phase1→2 延后到本帧尾清之后」处理。
- 门链/电梯链未再改：door 6/6 的达标是靠「载房起拍件序全跑、窗后续写拍才 dry」这一分拍判据。

### 9.11.5 参考图代际固定与 21 案矩阵（2026-11-24，回应 #27 退回补证）

对影退回的两条都是「证据面不可复现」（参考图代际未固定、缺 21 案表）。本节补上，口径：

#### 9.11.5.1 参考图代际（`sched_sha256`）

本轮全部参考图用 **当前树 + #27 `oam.rs` 补丁** 的 `run-all --fresh-emu` 重采；矩阵文件
`shots_rust/matrix.json` 记每案 `provenance.rust/emu = fresh`。关键三案的调度表 sha：

| 用例 | `sched_sha256` | 采集点 | 像素结果 |
|---|---|---|---|
| `case-mechanism-door-transition` | `c4f47612887926f8162d37152cf5be423424d12598f0b34c1e8c564397b74be8` | 6 | 6/6 = 0 |
| `case-mechanism-input-conflict` | `d00f500e22842e55…` | 11 | c674=84，其余 0 |
| `case-scrollseam-probe` | `6a4d8e9de5945f1b7dedf9eb16a248f87cdeae71afc9087ba2e69ba051fcc981` | 14 | 14/14 = 0 |

door 六格参考图 png sha256（前 16 hex）：

| 采集点 | 帧 | diff | png sha256[:16] |
|---|---|---|---|
| approach | 470 | 0 | `73838fc23416642b` |
| door-trigger | 481 | 0 | `d67774b21e79e17e` |
| load-chain | 486 | 0 | `dd3d0abd89e33386` |
| transition | 500 | 0 | `6204d7539fbb7ee6` |
| in-room | 560 | 0 | `a6a8d2ee93a83279` |
| end | 600 | 0 | `7236c2b74b155e86` |

#### 9.11.5.2 21 案逐案表（`run-all --fresh-emu`，blocked=0）

用例 21 · 采集点格 172 · **match 170 · diff 2 · blocked 0**（pass 170/172）。
两处 diff 均是既有分歧、不属本模型：`case-mechanism-input-conflict c674=84`（`$FE` 早一 tick）、
`case-password-justin f1191=2063`（同族，见 §9.10 记录）。

| 用例 | 采集点 | 一致 | 差异 | blocked | sched_sha256(前12) |
|---|---|---|---|---|---|
| `case-001-game-start-samus-interaction` | 13 | 13 | 0 | 0 | `94fec4a3308a` |
| `case-002-door-blue-interaction` | 8 | 8 | 0 | 0 | `5f6b7f1d07a5` |
| `case-collide-probe` | 7 | 7 | 0 | 0 | `b77500f7647a` |
| `case-crawlprobe` | 6 | 6 | 0 | 0 | `35709769dace` |
| `case-elev-norf-room-load` | 12 | 12 | 0 | 0 | `a351e348d6c8` |
| `case-elev-norfair-transition` | 10 | 10 | 0 | 0 | `c4636e892899` |
| `case-fcprobe` | 2 | 2 | 0 | 0 | `0483ad824afd` |
| `case-jumpledge-probe` | 8 | 8 | 0 | 0 | `784ca2c9aa0c` |
| `case-killenemy-probe` | 7 | 7 | 0 | 0 | `729bc76579f2` |
| `case-leftwalk-probe` | 6 | 6 | 0 | 0 | `98c7227aeb19` |
| `case-leftwalk-probe2` | 7 | 7 | 0 | 0 | `700e33033838` |
| `case-mechanism-door-negative` | 3 | 3 | 0 | 0 | `32569cc367cd` |
| `case-mechanism-door-transition` | 6 | 6 | 0 | 0 | `c4f476128879` |
| `case-mechanism-input-conflict` | 11 | 10 | 1 | 0 | `d00f500e2284` |
| `case-mechanism-morphball` | 4 | 4 | 0 | 0 | `3edafbd3823a` |
| `case-morphball-probe` | 21 | 21 | 0 | 0 | `6a2c7bdccbd8` |
| `case-morphball-tail-probe` | 4 | 4 | 0 | 0 | `0b46dc2ade2c` |
| `case-password-justin` | 14 | 13 | 1 | 0 | `d4b8cdefe466` |
| `case-scrollseam-probe` | 14 | 14 | 0 | 0 | `6a4d8e9de594` |
| `case-shootprobe-probe` | 4 | 4 | 0 | 0 | `e14064d10d77` |
| `case-walkfcprobe` | 5 | 5 | 0 | 0 | `1e5ec9ce15a6` |

#### 9.11.5.3 机制归属（未定谳，但已收窄）

织码在同一场景（`probe-scrollseam-win`）给出的逐字节数据：f1100 的 `$0200-$02FF` 只差 7 字节
（`$0217/$021B/$021F/$0223/$0227/$022B/$022F`，全是 Samus 槽 5–11 的 x，各 −1），且 f1097/f1101/f1102
的 OAM 全 0 差；状态面 `$030E`（SamusX）/`$FD`（ScrollX）两边逐帧同值：

| 帧 | SamusX | ScrollX | 期望 x=(X−scroll)&0xFF | emu OAM | rust(HEAD) |
|---|---|---|---|---|---|
| f1097 | 142 | 255 | 143 | 143 | 143 |
| f1098 | 143 | 1 | 142 | 143（窗内不写，留旧值） | 142 |
| f1099 | 143 | 1 | 142 | 143（同上） | 142 |
| f1100 | 144 | 1 | 143 | 143 | 142 |
| f1101 | 145 | 2 | 143 | 143 | 143 |

⇒ 与本次落码相容的解释：**窗内那两拍 rust 提前写、恢复拍又沿用旧 x**；`dry` 实现把窗内写推迟，
使 f1100 用当帧 SamusX 重算 ⇒ 7 枚精灵 x 回到 143。
「游标空推进 7 槽」与「两轮 emu 页不同」两条旧解释**均已作废**（两份 ram.hex 整页 sha 相同 `699cbd3e…`；
按 `oamwrites.jsonl`，f1098 写 0 次而 f1099 写 128 次，所以「窗内一律不写」也不准确）。
x−1 的逐字节表本身是 A 级；实现形的语义归属仍记 C 级，待另开卡按 `oamwrites` 时间线定谳。

## 10 V2 取证：帧内执行路径（PC/循环计数，#30）

### 10.1 方法与命令

只读探针（不改 `crates/`、不写 `shots/`）：

```sh
python3 analysis/_probe/v2_pc_path.py       # Mesen GUI + Xvfb，挂 $F345 exec 回调，记 emu.getState()["cpu.cycleCount"]
```

产物（`analysis/_probe/v2pc/`，本地可用、不入库）：

| 文件 | 内容 | 字段 |
|---|---|---|
| `cycles.tsv` | 每帧 `UpdateAllEnemies($F345)` 入口的 CPU 周期计数 | `frame`、`cycle` |
| `pcpath.tsv` | 每帧各敌例程 exec 次数 | `frame,F345,F351,F37F,F3BE,F3E6,F410,E094` |
| `statekeys.txt` | `emu.getState()` 可用键清单（自证 `cpu.cycleCount` 存在） | 每行一键 |

数据源与既有 trace 同一输入：`analysis/_gen/_tmp_sweep_1629615-1795445195_a2_r29.lua`（Kraid29 窗口 1650–1744 的 sweep）。
判定口径沿用 #25：列边界帧 = `ScrollX % 8 == 0` 且 `ScrollX` 比上一帧大；超预算 = 该帧 `ScrollX` 增量 0（停）。

### 10.2 逐边界帧表（20 帧）

「迭代周期」= 相邻两次 `$F345` 入口的 `cpu.cycleCount` 差（即上一次引擎迭代的真实耗时，起于 f−1）；
「跨帧」= 该差覆盖的帧数（≥2 = 中间有帧没进引擎迭代）。帧预算 `FRAME_CYCLES = 29780`。
最后一列是**该迭代内部**的七路敌例程计数（`pcpath.tsv` 的 f−1 行）——停帧本帧计数全 0 是同义反复（§10.3①），
用迭代内计数才有真实的路径证据。

| f | ScrollX | 判定 | 迭代周期（起于 f−1） | 跨帧 | 该迭代内七路计数 F345/F351/F37F/F3BE/F3E6/F410/E094 |
|---|---|---|---|---|---|
| 1650 | 96 | 便宜 | 27953 | 1 | 1/6/6/3/0/0/3 |
| 1654 | 104 | 便宜 | 27955 | 1 | 1/6/2/0/0/0/0 |
| 1658 | 112 | 超预算(停) | 57734 | 2 | 1/6/6/6/0/0/6 |
| 1663 | 120 | 超预算(停) | 57718 | 2 | 1/6/6/6/0/0/6 |
| 1668 | 128 | 超预算(停) | 57736 | 2 | 1/6/6/6/0/0/6 |
| 1673 | 136 | 超预算(停) | 57491 | 2 | 1/6/6/6/0/0/6 |
| 1678 | 144 | 超预算(停) | 57736 | 2 | 1/6/6/6/0/0/6 |
| 1683 | 152 | 便宜 | 28199 | 1 | 1/6/6/6/0/0/6 |
| 1687 | 160 | 便宜 | 27929 | 1 | 1/6/6/6/0/0/6 |
| 1691 | 168 | 超预算(停) | 57734 | 2 | 1/6/6/5/1/1/6 |
| 1696 | 176 | 便宜 | 27949 | 1 | 1/6/6/5/1/1/6 |
| 1700 | 184 | 超预算(停) | 57503 | 2 | 1/6/6/5/1/1/6 |
| 1705 | 192 | 超预算(停) | 57478 | 2 | 1/6/6/5/1/1/6 |
| 1710 | 200 | 超预算(停) | 57711 | 2 | 1/6/6/5/1/1/6 |
| 1715 | 208 | 超预算(停) | 57731 | 2 | 1/6/6/5/1/1/6 |
| 1720 | 216 | 超预算(停) | 57736 | 2 | 1/6/6/5/1/1/6 |
| 1725 | 224 | 超预算(停) | 57727 | 2 | 1/6/6/5/1/1/6 |
| 1730 | 232 | 超预算(停) | 57968 | 2 | 1/6/6/5/1/1/6 |
| 1735 | 240 | 超预算(停) | 58265 | 2 | 1/6/6/5/1/1/6 |
| 1740 | 248 | 便宜 | 27952 | 1 | 1/6/6/5/1/1/6 |

### 10.3 被否证的三条特征（含反例）

1. **敌例程路径计数（本帧）**：超预算帧的 `F345=F351=F3BE=F3E6=E094=0`，便宜帧全部非 0
   ⇒ 表面上完全可分，但这是**同义反复**：帧被跳过时自然没有任何迭代执行。不能当解释。
2. **上一帧的路径计数**：反例成对出现——`f1657` 与 `f1682` 的计数向量逐项相同
   `(1,6,6,6,0,0,6)`，但 f1658 是超预算、f1683 是便宜；`f1695` 与 `f1739` 同为
   `(1,6,6,5,1,1,6)`，f1696 便宜、f1740 便宜而 f1691/1700…1735 超预算 ⇒ **该特征无效**。
3. **名册/类型**：f1658–1740 之间 13 个边界帧名册恒为 6 只 `[8,9,9,9,10,10]`
   （`EnsExtra` 每帧逐字节比对），却 5 便宜 9 超 ⇒ **类型权重与名册规模都不足以定案**（同 #25 结论）。

### 10.4 站得住的可证伪判据：迭代耗时

把「迭代周期」当观测量，判据写死为：

> 若某次引擎迭代的 `cpu.cycleCount` 差 > 1.5 × `FRAME_CYCLES`（≈ 44 670），
> 则该迭代跨 ≥2 帧，下一帧 `ScrollX` 不推进（停）。

回代样本口径（修正）：**窗口 1604–1755**。1600–1603 是到达相位（`FC` 冻结为 46、`ScrollX` 2→3→3→3、
停由载房到达造成），不计入滚动预算样本；1729 帧之后的 1744/1745 是门切换相位
（`DoorEntryStatus` 在 1744 变 5、`ScrollX` 归 0），也不计。

- 样本内「跨 ≥2 帧」迭代 **15 次**（gap 恒为 2），全部被判据命中：**precision = 1.000，recall = 1.000（15/15）**；
- 20 个边界帧的「超预算/便宜」全部由该判据给出（14/14 停、6/6 便宜，见 §10.2）；
- 耗时分布双峰：正常迭代 27 925–29 784，每 5 帧一次的 32 300–32 600（单帧内完成），
  以及 57 500–58 300（跨 2 帧，停摆唯一来源）。

**判据性质（如实标注）**：`cycleCount` 差只能在两次 `$F345` 入口之间测得，因此「差≈2 帧预算 ⇒ 跨 2 帧」
在算术上接近同义重述，它是**机制描述**、不是前向预测。真正的前向预测式（用上一迭代累计周期预判下一帧是否停）
本卡未建立，属 V2 下一步（与 §10.5 的拆分同批做）。

### 10.5 仍是缺口（不写成已验证）

- **为什么某些迭代会跨 2 帧**尚未分解：本卡只证到「耗时双峰 + 判据可用」，没有把 57.5k–58.3k 拆成
  「名字表列流送 + 敌人 + 其他件序」各占多少。下一步需要按例程入口/出口分别记 `cycleCount`
  （`$F345`、`$CB6A` 尾清、`CheckUpdateNameTable`），本卡未做。
- **Brinstar21 / Norfair03 对照房：未取证**。当前 `analysis/traces/_tmp_sweep_1109309…a0_r21` 与
  `_tmp_sweep_1155643…a1_r03` 的 `frames.jsonl` 只有 8 字段、没有 `FC`/`cpu.cycleCount`，无法套用本判据。
  V2 补采命令（需要 emu，且要先确认全机无 Mesen 实例）：

  ```sh
  python3 tools/movie_ref/room_sweep.py --area 0 --room 21 --frames 400 --capture 200:mid --capture 400:end
  python3 tools/movie_ref/room_sweep.py --area 1 --room 3  --frames 400 --capture 200:mid --capture 400:end
  ```

  需要的字段：`watch 002d`（FC）、`watch 0014`（Joy1）、`ScrollX`、`EnsExtra`，外加本卡的
  `cpu.cycleCount` 逐迭代采样（`analysis/_probe/v2_pc_path.py` 可直接复用，把 `GEN` 指到对应 lua）。
- 本卡**不含**任何「7 房全修完」的声明；§7/§7.1 旧表一字未改（本文件当前尚未落库，无法给 `git diff`
  佐证；§7 在 :314–:329、§7.1 在 :330–:395（本次追加前的位置），PMO 落库后可用 `git log -p` 对该区间做一次 mechanically 对照）。
