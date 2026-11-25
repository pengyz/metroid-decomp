# Stage A 通关验收面（节点判定表 + 门禁设计）

> 卡：#69。写作时 HEAD `cde7d1b9`（2026-11-25）。
> 本文只定义**验收面**与**判定规则**，不改代码；修断链另开卡。
> 证据分级沿用 AGENTS.md §3（A=字节/汇编可复现，B=代码语境推断，C=假设）。

## 0. 结论速览

| 问题 | 回答 | 依据 |
|---|---|---|
| 现在能通关吗？ | **不能** | 静态链路 5 条断 2 条（§1 L1/L2） |
| 断在哪一步？ | 玩家打死 Boss 之后：击杀结算不写 `$687B/$687C`，进门也不读它 ⇒ 雕像永不转闪烁态 ⇒ 桥不解锁 ⇒ 到不了 Tourian | §1 L1/L2，`docs/spec/game-progress.md` §1.4/§1.2 |
| L4 171/172 全绿说明什么？ | 只说明**已有的 21 个采集点**像素一致，不说明链能走通 | §3 |
| TAS 只到 f22895（19%）说明什么？ | 说明「已跑过的 2.3 万帧」行为一致，其余 81% 未验 | `docs/findings/tas-100-round3.md` |

一句话：**今天所有绿灯都是局部一致性问题，没有任何一个门禁回答「玩家能不能打到结局」。** 本文把这个面建起来。

## 1. 判定表（必经节点 × 当前状态）

节点顺序 = 必经顺序。`状态` 三值互斥：**已验证**（Rust 侧有实现且本文读到对应代码）/ **断链**（实现缺失或语义不一致，给 `文件:行`）/ **未验证**（实现存在但无动态证据）。

| # | 节点 | 判定量 | 状态 | Rust 落点 / 证据 |
|---|---|---|---|---|
| S1 | 首个 Boss 击破（在 Boss 区） | `$687B bit0` @区2 或 `$687C bit1` @区4 | **断链** | `crates/game-core/src/enemy/framework.rs:520 enemy_complete_death` 无 `prevStatus bit7 → LDD75` 分支（A，`docs/spec/game-progress.md` §1.4） |
| S2 | 两个 Boss 均击破 | `$687B bit0` 且 `$687C bit1` | **断链** | 同上（同一处缺失导致两个状态都写不进） |
| S3 | 两座雕像升起 | 两字节 `bit7` 且在本区（Brinstar） | 已验证 | `crates/game-core/src/world/statue.rs:93`（命中后逐帧上移 15px，到低位 0 置 bit7） |
| S4 | 通路桥生成 | `$0054 == 1` | 已验证 | `crates/game-core/src/world/statue.rs:38`（两座 bit7 ⇒ 置位 + 8 个 TileBlast） |
| S5 | 进入 Tourian | `$0074 & 0x0F == 3` | 已验证 | 电梯 `items.asm it_Elevator,$03`（`docs/spec/game-progress.md` §0#6）；`world/elevator.rs` |
| S6 | Zebetite 清空 | `$0758+8*i` 五座 `status` 全 0 | 未验证 | `crates/game-core/src/enemy/zebetite.rs`（5 座、8 击、64 帧自愈 1 击）；无 Tourian 像素证据（`room-sweep.md` §8.1） |
| S7 | 母脑击破 | `$0098 == 7`（11 态表死亡态） | 未验证 | `crates/game-core/src/enemy/mother_brain.rs`；死亡态序号为 B 级（11 态表未逐态对拍） |
| S8 | 逃逸计时耗尽 | `$010A` 曾 >0 后归 0 | 已验证 | `crates/game-core/src/world/end_timer.rs` |
| S9 | 结局触发 | `$6883 != 0` | 已验证 | `crates/game-core/src/world/elevator.rs:18`（`etype==0x8F`）；`title/mod.rs:1941 choose_ending` |

**另有一条独立断链，不属玩家必经节点但影响「回头再战」**：

| # | 连接点 | 状态 | 证据 |
|---|---|---|---|
| L2 | Boss 房重生门：进房时读 `$687B/$687C` 决定是否跳过生成 Boss 记录 | **断链** | `crates/game-core/src/world/room_draw.rs:823 load_enemy` 不读这两个字节（A，§1.2） |

> 为什么这是「断链」而不是「未验证」：Rust 侧该函数体已逐行读过（`enemy_complete_death` 走「status←Pickup / data0D←$60 / 1-16 无掉 / PickupTypeTable」全径，**没有任何一处读写 `$687B/$687C`**），是**实现缺失**，不是缺证据。

## 2. 门禁设计：两条候选 + 推荐

### 候选 A：脚本化通关回放（按键序列驱动 + 节点判定）
- 输入=按键序列（`cases/*.txt` / `.inputlog`），跑 Mesen 侧得逐帧状态，按 §1 节点表判定「到达哪些节点、第一个断点在哪」，要求**全节点按序到达**才算通过。
- 成本：需要一个**能打到结局的输入序列**（现有最长只到 Norfair 出生段；TAS 整片也不含通关）。
- 优点：能给出**人类语义**的失败点（「断在打死 Kraid 之后」），且与现有 `cases/` 管线同格式。

### 候选 B：100% TAS 整片 120540 帧零分歧作为唯一一等门禁
- 输入=既有 TAS 录像，要求 `first_divergence == null`。
- 成本：当前只对齐到 f22895（19%）；且**整片零分歧蕴含通关行为一致，但它不告诉你在哪断**——一旦红，定位仍要靠节点表。
- 优点：不需新造输入，纯机械，无法「挑好路走」。

### 推荐：**A 做一等、B 做强约束，两者都进 M 系列门禁**
理由：
1. **A 给结论、B 给信心。** A 的产出是「当前能打到哪一步」这一句人话，这正是对影在会议上指出的缺失；B 的产出是「跑过的部分逐字节一致」，它天然无法回答「能不能通关」。
2. **A 的输入成本是一次性的**（一条打到结局的序列，可长期复用、可被 B 交叉验证），而 B 每次都要 37 分钟，不适合当提交闸。
3. **A 的判据是互斥的**（节点全到达 = 通过；任一节点未到达 = 不通过，并给出第一个未到达点），B 的判据是「零分歧」，两者不重复。

**分工（写死，避免以后两边互相顶替）**：
| 面 | 角色 | 频率 | 失败含义 |
|---|---|---|---|
| 静态链路 `stage_a_nodes.py links` | **提交闸**（秒级、纯静态） | 每次 `check` | 链在某处断了 → 不许合入「已完成通关链」的声明 |
| 动态节点 `stage_a_nodes.py trace` | **一等验收**（每次 M 里程碑） | 里程碑/发版 | 玩家在某个节点过不去 → Stage A 未完成 |
| TAS 整片零分歧 | **强约束/回归**（周级，37 分钟） | 周级 | 已对齐段出现行为漂移 → 必须修 |

## 3. 与现有验收面的关系（谁一等、谁回归）

| 面 | 覆盖 | 当前读数 | 在本文框架里的角色 |
|---|---|---|---|
| L4 矩阵 | 21 案 172 个采集点像素 | 171/172（唯一 c674=84） | **回归**：防止已对齐的画面漂移；不证明链通 |
| `room_sweep` | 逐房加载/滚动 | 见 `docs/findings/room-sweep.md` | **回归**：房间级一致性 |
| 门禁 24 项 | 命名/裸地址/账本/测试数等 | PASS=24 | **卫生**：与「能不能通关」正交 |
| 本文节点表 | 主菜单→结局 9 节点 | S1/S2 断链 | **一等验收**：唯一回答「能通关吗」的面 |

**判定规则（互斥，可直接代入复核）**：
1. `links` 任一条断 ⇒ 「进度链不完整」，任何「通关链已完成」的声明无效。
2. `trace` 首节点未到达 ⇒ 出「停在 S1」结论，不得用 L4 绿代替。
3. L4 全绿但 `links` 有断 ⇒ 仍判不通关（今天就是这个状态）。
4. TAS 零分歧但缺通关序列 ⇒ 仍不能声称通关（只声称「已跑部分一致」）。

### 与现有 TAS 管线的复用点与边界

| 复用 | 说明 |
|---|---|
| 输入格式 | `{f, w}` JSONL —— 与 `tools/scenario_run.py` 的 `analysis/traces/<name>.frames.jsonl` 完全同形，也与 `tools/tas_replay_diff.py` 读取的 `rust.state.jsonl` 同形（它内部同样取 `d.get("w")`） |
| 采集侧 | 不另造回放器：Mesen 侧用 `scenario_run` 的 `watch <addr> <label>` 直接产出节点所需的 9 个键；Rust 腿由 `tas_replay_diff.py` 的既有 state dump 提供 |
| 判定侧 | 新写的只有「节点谓词 + 到达表」这一层（本工具），复制不了也替代不了 `tas_replay_diff.py` 的逐帧对拍 |

**边界（明确不做什么）**：
1. 不做按键驱动、不启模拟器、不比对像素——输入日志/仿真归 `tas_replay_diff.py` 与 `scenario_run`。
2. 不判定「同一个值是对是错」——那是对拍的事；本工具只判「节点到了没有」。
3. 不代表 emu/rust 两侧必须给出同一份轨迹：两侧各跑一份，比较的是**节点到达表**（这才是通关语义）。

## 4. 首版入口（`tools/stage_a_nodes.py`）

```bash
python3 tools/stage_a_nodes.py nodes                    # 打印节点表/链路表/watch 键
python3 tools/stage_a_nodes.py links                    # 静态链路（秒级）
python3 tools/stage_a_nodes.py trace --trace <frames.jsonl>   # 动态到达表
python3 tools/stage_a_nodes.py report --trace <frames.jsonl>  # 合成一句判决
python3 tools/stage_a_nodes.py selftest                 # 断言组 6
```

实测（HEAD `cde7d1b9`，2026-11-25）：

```
$ python3 tools/stage_a_nodes.py links
[链路] 进度链静态校验：5 条，断链 2 条
  [断链] L1 Boss 击杀结算写雕像状态 —— 缺模式：0x687B|KRAID_STATUE_STATUS
  [断链] L2 Boss 房重生门读雕像状态 —— 缺模式：0x687B|KRAID_STATUE_STATUS
  [OK  ] L3 雕像升起 —— crates/game-core/src/world/statue.rs:93
  [OK  ] L4 桥生成 —— crates/game-core/src/world/statue.rs:38
  [OK  ] L5 结局电梯触发 —— crates/game-core/src/world/elevator.rs:18
$ echo $?
1
```

动态面用真实轨迹验过（`analysis/traces/probe-stage-a.frames.jsonl`，Mesen 采 1200 帧，
`python3 tools/scenario_run.py run /tmp/probe-stage-a.txt` → `frames: 1200 captures: 40 wall: 31.3s`）：

```
$ python3 tools/stage_a_nodes.py trace --trace analysis/traces/probe-stage-a.frames.jsonl
[轨迹] … 帧数 1200
  S1 首个 Boss 击破（在 Boss 区）   —   (A)     ← 第一个断点
  …（S2–S9 全 —）
```

Rust 腿也能吃同一张节点表：`tas_replay_diff.py` 的 Rust 腿产物 `rust.state.jsonl` 里
`w={zp,iram,wram,pw}` 是整块内存 hex，工具按声明的偏移（`kraid_status=$687B` 在 wram+0x87B、
`in_area=$0074` 在 zp+0x74、`end_timer=$010A` 在 iram+0x10A…）换算，缺块或越界就按「未采集」处理
（不臆造成 0）。实测 `analysis/tas-m1/t33-full/rust.state.jsonl`（28580 帧）：
`到达 0/9 个节点，第一个断点 S1`，与 emu 腿结论同向。

> 边界：blob 模式只做过**结构自证**（`selftest` 用已知字节构造 zp/iram/wram 并断言取出的值，
> 另断言越界块按缺省），**没有**与同一轮 emu 腿逐帧比对过数值。要把它当门禁用，先在某一轮
> 长跑里做一次「同帧 emu 命名键 vs rust blob」抽检。另注意 iram 只覆盖 `$0000-$06FF` 时
> 取不到 `$0758` 起的 Zebetite（工具会判「未采集」而不是「已清空」），这是 fail-closed 行为。

**这条轨迹揭露了一个判定表必须修的坑（已修，留证）**：同一份轨迹首版跑出来是
`S1/S2/S3 全部在 f1191 到达`——因为 JUSTIN BAILEY 密码在出生时就写了状态字节：
`$687B=0x81`、`$687C=0x82`（bit0/bit1 = 已击败，bit7 = 雕像已升起），而 `$0074=0x11`（Norfair）。
若节点只看字节值，**任何带密码开局的轨迹都会把「击杀→雕像」整段误判为已达成**。
修法：S1/S3 加**区域门**（S1 要求 `InArea&0x0F∈{2,4}`，S3 要求 `=0`），并在 `selftest` 里补了
「缺区域上下文 ⇒ 不算到达」的断言。

另一条已写进工具的判据：**节点只在它引用的全部 watch 键都出现在该帧时才判定**，
避免「没采集的键默认 0」把未观测误判成达成（`selftest` 有专门断言守这条：
`Zebetite 未采集 ⇒ 断在 S6`）。

## 5. 缺口与后继

| 缺口 | 影响 | 建议 |
|---|---|---|
| 没有一条打到 Boss 的输入序列 | 动态面只能证明「到不了 S1」，不能证明「打死 Boss 后雕塑不亮」 | 开一张「造第一条 Boss 击杀序列」卡（只做输入，不改代码） |
| L1/L2 两处断链 | 通关链物理断开 | 已有 `docs/spec/game-progress.md` §1.4/§1.2 的逐指令规格，可直接派实现卡（写面 `crates/game-core/src/enemy/` + `world/`） |
| S6/S7 判据为 B 级 | Zebetite 清空与母脑死亡态的观测值未逐态对拍 | 后继在 Tourian 场景拿到像素/OAM 证据后升 A |
| `$010A` 在密码进场后读 0xFF | 疑未初始化哨兵，S8 的 `require_seen` 已规避误判 | 留在 `docs/spec/game-progress.md` §6 假设区，不在此处下结论 |
