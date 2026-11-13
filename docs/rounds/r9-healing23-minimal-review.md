# 止血件 2/3 独立对抗复核（证伪立场，非实施方）

- 复核人：verifier（独立，未参与实施）
- 日期：2026-11-13
- 对象：`5ff70e3c`（止血件2 载房掉帧窗五点校准）、`d154544a`（止血件3 GFX 烘焙去重）
- 手段：**纯 python 只读验算 + 读文件 + git show**（未跑 cargo test / gates cli / rust_screenshot / L4 矩阵）

---

## 一、结论速览

| 事 | verdict | 一句话 |
|---|---|---|
| 事一 `5ff70e3c` | **needs_revision**（数字 pass / 证据链 pass 不足） | 我独立重算的区间、中点、偏差**与声称逐字一致**；但「五点 emu 实录」的 x 坐标（work_cycles）**不是 emu 实测值**，且 5 点里 2 点无任何在库可复现指针、trace 本身未入库、文档有 1 处数值错误（28666c） |
| 事二 `d154544a` | **pass**（附 3 项非阻断问题） | 三份等价性**独立实测全 True**（203B / 29 项 / 13 块 / 旧 match 13 臂）；29 项 ROM 逐字节、`sum==len`；语义变更为「潜在扩展」，提交树内**零行为变化成立**——但工作树已有未提交补件**依赖该扩展**，且「三遍」实为**四遍** |

---

## 二、事一：`5ff70e3c` 载房掉帧窗五点校准

### 2.1 第 1 步：抄出实际写的五点（`crates/game-core/src/frame/room.rs:1245-1251`）

```rust
const POINTS: [(u32, u32, &str); 5] = [
    (31446, 2, "room20（case-002 邻房）"),
    (36620, 2, "room19（probe-loadwin f1067）"),
    (43378, 2, "room9（probe-loadwin f1286）"),
    (46456, 2, "room23（morphball 走廊）"),
    (71946, 3, "room8（morphball 球房）"),
];
```

**与声称完全一致**（五点元组逐字相同）。常量：`LOAD_BASE_CYCLES: u32 = 29000`（room.rs:66）、
`FRAME_CYCLES: u32 = 29780`（room.rs:67）。

### 2.2 第 2 步：丢掉断言代码，自己解区间（独立实现）

公式 `dropped = (BASE + work) / FRAME_CYCLES`（整数除法），对每点 `d ≤ (BASE+work)/F < d+1`
⇒ `d*F - work ≤ BASE < (d+1)*F - work`：

| 点 | work | d | 自算 BASE 区间 |
|---|---|---|---|
| room20 | 31446 | 2 | **[28114, 57894)** |
| room19 | 36620 | 2 | [22940, 52720) |
| room9 | 43378 | 2 | [16182, 45962) |
| room23 | 46456 | 2 | [13104, 42884) |
| room8 | 71946 | 3 | [17394, 47174) |
| 常规帧 | 0 | 0 | [0, **29780**) |

- 下界 = max = **28114**（唯一来自 room20）→ 与声称一致
- 上界 = min = **29780**（唯一来自 work=0）→ 与声称一致；room8 的 47174 更松，**无贡献**
- **交集 = [28114, 29780)**，宽 1666c = **5.594%** 帧（声称 5.6% ✓）
- 中点 = (28114+29780)/2 = **28947**；29000 − 28947 = **+53**（声称 ✓）
- 29000 在区间内：真；距下界 +886（2.97%）、距上界 −780（2.62%）——**文档这三个数字全部复核通过**
- 逐点回代 BASE=29000：五点全部命中，`work=0 → 0` 命中

> **算术面无异议。区间、中点、偏差全部自算复现。**

**补充（我的独立观察，非实施方所述）**：5 点中**只有 2 点真正参与夹逼**——room20 定下界、
work=0 定上界；room19/9/23/8 四点对交集**零贡献**（room9 的下界 16182、room23 的 13104 远低于
28114；room8 的上界 47174 远高于 29780）。文档 §3.1 只解释了 room19/9 零贡献，
未说明 room23/room8 同样零贡献，标题「五点 emu 实录」容易被读成"五点共同夹逼"。

### 2.3 第 3 步：文档数字与 2.1/2.2 的一致性

`docs/findings/load-window-multipoint-calibration.md` §3 表格 **6 行全部与我自算相同**
（区间端点、宽 1666c / 5.6%、+886 / −780 / +53）。§5 的容差换算也复核通过：
780/9262 = 8.42%（文中 8.4%）、780/349 = 2.23 个元 tile（文中"约 2 个"）、780/300 = 2.6 个敌人 ✓。

**但发现 1 处数值错误（needs_revision 的直接依据）**：

- `docs/findings/load-window-multipoint-calibration.md:98`：「下界由 **room20** 单独确定（**28666c** 的 work 就要 2 帧掉帧）」
- 实际：BASE=29000 时要掉 2 帧需 `work ≥ 2*29780 − 29000 = **30560**`；在区间下沿 BASE=28114 时需
  `work ≥ **31446**`（即 room20 自身）。
- **28666 在任何区间内取值下都不可能掉 2 帧**（28666 < 29780，连一帧的余量都不足），且与本文件
  §3 表格里 room20 的 `work=31446` 自相矛盾。属笔误/未复核数字，虽不影响区间结论，但违反
  「每个数字要有亲手跑出的支撑」。

### 2.4 第 4 步：反证——这五点的来源与可复现性

**（a）x 坐标 `work_cycles` 不是 emu 实测值（最重要的定性问题）**

`work_cycles` 在 Rust 侧是**模型累加量**，由 room.rs:116/120/158/173/509/632 与
mainloop.rs:3145 的 `LOAD_*` 常量累加得出：

```
grep -rn "work_cycles" crates/  →  仅 physics.rs:1373 置 0、room.rs/mainloop.rs 累加、
                                   physics.rs:1384 读用；无任何 emu/RAM 来源
```

即：**y 坐标（dropped）是 emu 可观测的，x 坐标（work）是模型输出**。测试注释
`// 五点 emu 实录（work_cycles, 实测掉帧数）`（room.rs:1244）把这一对整体标为"emu 实录"，
属**表述夸大**：它实为「模型算出的 work × emu 实测的 dropped」。这在方法上并非致命（校准 BASE 正是
以此为输入），但意味着该区间**条件于另外 6 个未独立校准的 `LOAD_*` 常量**，不是 emu 直接测量的区间。
文档 §2 起首亦承认「`LOAD_BASE_CYCLES` 不可直接测量」，但未明说 work 同样不可直接测量。

**（b）5 点中仅 3 点可在库内复现；另 2 点无指针**

库内 trace `analysis/traces/probe-loadwin.frames.jsonl`（2400 帧）我逐帧解析 `$2D`(FC) 差分：

```
dFC 分布：{0: 20, 1: 2370, -255: 9}   （-255 = FC(u8) 255→0 回绕；2399 个 delta 全对账）
dFC==0（游戏逻辑未推进=掉帧）的帧：f2 / f4-6 / f311-313 / f352-358 / f896-897 / f1068-1069 / f1287-1288
```

对照文档 §2.1 的「f1 停1 / f3 停3 / f310 停3 / f351 停7 / f895 停2 / f1067 停2 / f1286 停2」：
停滞**起点**偏移一位（我按 delta 计得到 f2/f4/f311/f352/f896/f1068/f1287），**停滞长度逐一吻合**
（1/3/3/7/2/2/2）。⇒ room20=2、room19=2、room9=2 三个 dropped 值**可在库内复现**。

- **room23（46456→2）与 room8（71946→3）在 `probe-loadwin` 里不存在**（该 trace 只出现房号
  0/9/19/20/240/241/243/244/255，**无 23、无 8**）。文档 §2 自述这两点来自「既有校准 3 个房间」，
  但**全文未给任何 trace id / 文件 / commit 指针**。
- 文档 §2.2 的 work 值来源自述为「临时探针 `[loadwin]`，**取证后已 `git checkout` 回退**」——
  **x 坐标的取证器不在库里，无法复跑**。

**（c）「A 级」证据指针本身未入库**

```
git ls-files --error-unmatch analysis/traces/probe-loadwin.frames.jsonl
  → error: 未匹配任何 git 已知文件      （.gitignore:19,24 含 analysis/）
```

trace 文件在 `analysis/`（AGENTS §2 明定「不入库」），场景文件在 `/tmp/probes/probe-loadwin.txt`
（文档 §8.3 自承未入库）。⇒ 文档 §2.1 标注的「A 级」证据，其**唯一文件载体既不版本化、也会被清理**。
按 AGENTS §3「A 级=字节偏移/汇编行/trace id」，这里缺的是**可持久定位的 trace id**。

**（d）文档自查已登记一处不一致（诚实项，不是隐瞒）**：§2.3 表 room9 行标 `❌ 早 28 帧`
（emu f1286-1288 vs rust span f1258-1259）。room9 的 (work, dropped) 对来自**错位 28 帧的那次运行**；
因该点对交集零贡献，数值上无害，但它是「rust 与 emu 首次 FC 分叉」的现成证据，文档归为「另案」——
我认可其登记，但提醒：**用它做校准输入时并未附加"该点已分叉"的置信折扣**。

### 2.5 事一小结

- **数字层面：pass**（区间 [28114, 29780)、中点 28947、+53、5.6%、+886/−780 全部自算复现）。
- **证据层面：needs_revision**——① `work` 非 emu 实测却被标为「emu 实录」；② room23/room8 无指针；
  ③ trace + 场景未入库，A 级证据不可持久复现；④ 文档 1 处数值错误（28666c）。
- 定性：这一条**不是"骗人的绿"**，区间是真的、也确实把旧注记的 [25850,29780) 收紧了
  （28114 vs 25850，宽度 3930→1666）；但它宣称的证据等级 **A 级偏高**，实为
  「emu 实测 dropped × 模型 work」的 **B 级条件约束**。

---

## 三、事二：`d154544a` GFX 烘焙去重

### 3.1 第 1 步：`GFX_INFO` 与 ROM 逐字节（自己解析源码数字）

```
rom/Metroid (USA).nes → prg = file[0x10:]（len 131072）
prg[0x1C6E0 : 0x1C6E0+203]  head = 0600800000a00904608d0000
GFX_INFO（gfx_data.rs，203B）head = 0600800000a00904608d0000
=> GFX_INFO == ROM 逐字节：**True**（203 字节中 0 处不同）
```

### 3.2 第 2 步：29 项 `GFX_DATA` 切片 vs ROM

按 `bank*0x4000 + src − 0x8000` 求 ROM 偏移，逐项比对：

```
ALL 29 byte-equal to ROM: True      （0 项失配）
sum(sizes) = 23712 = sum(GFX_OFFS 尺寸) = GFX_DATA.len() = 23712   → True
OFFS 连续性（off[i]+size[i] == off[i+1]）→ True
```

抽样（含全部 13 个在用 id）逐项 True：id0 2464 / id4 1104 / id20 256 / id21 1280 / id22 64 /
id23 1024 / id24 160 / id25 64 / id27 1968 …… 见脚本输出，**29/29 True**。

### 3.3 第 3 步：`gfx_blocks.rs` 13 常量 vs `GFX_DATA[GFX_OFFS[id]]`

```
13/13: True   （SAMUS/INTRO_SPRITES/TITLE/SOLID_1FC0/FONT_COMPLETE/FONT_HUD/SOLID_0FC0/
                SAMUS_SUITLESS/CREBG1/CREBG2/NORF_BG1/NORF_BG2/NORFAIR_SPRITES 全 bytes_eq=True、len_eq=True）
```

`gfx_blocks` 确已无人引用（仅 `crates/game-data/src/render/mod.rs:3 pub mod gfx_blocks;` 使其仍被编译；
`tools/bake_gfx_blocks.py` 亦然）——**"无人引用但仍存在"的陈述成立**。

**追加第四份等价性（实施方未验，我补验）**：被删掉的 `gfx_info` 手写 13 臂 match
（`git show d154544a^:crates/game-core/src/frame/gfx.rs`），我解析源码里的数字与
`GFX_INFO[id*7..id*7+7]` 比对：

```
arms = [0,4,5,7,8,9,20,21,22,23,24,25,27]（13 条）
逐臂 == GFX_INFO：True   （mismatched: []）
```

⇒ **三份（实为三处）拷贝逐字节等价，全 True。**

### 3.4 第 4 步（关键判据）：16 个新增 id 有没有调用点？

新实现 `gfx_info`：`id*7+7 <= 203` 即 **id ≤ 28 全部放行**；`gfx_block`：`GFX_OFFS.get(id)` 即 **0..=28**。
旧形 13 臂，其余 `_ => unported!`。**新增可达 id = 16 个**：
`1,2,3,6,10,11,12,13,14,15,16,17,18,19,26,28`（与声称的 16 个数量与集合一致）。

**调用图实测**（`grep -rn "load_gfx(" crates/`，`load_gfx` 是 `gfx_info`/`gfx_block` 的唯一生产消费者）：

| 调用点 | id 集合 |
|---|---|
| `frame/gfx.rs:68` `init_norfair_gfx`（生产，boot.rs:268 调） | 4,5,7,8,9,25,22 |
| `frame/gfx.rs:84` `load_samus_gfx` | 0,[27],20,23,24,25,22 |
| `frame/gfx.rs:97` `init_title_gfx`（生产，boot.rs:314 调） | 21,0,[27],20,23,24,25,22 |
| 测试 `frame/gfx.rs:211` / `boot.rs:589` / `boot.rs:603` | 21 / 5 / 0 |
| **HEAD 的 `boot.rs::init_bank1`** | **不走 `frame::gfx::load_gfx`**——用本文件内的**第 4 份副本** `boot.rs::load_gfx(mem, id: usize)`，直接读 `gd::GFX_INFO[id*7]`/`gd::GFX_OFFS[id]`，id = 3,4,5,6,25,22 |

**结论（提交树 d154544a 时点）：生产可传 id 并集 = {0,4,5,7,8,9,20,21,22,23,24,25,27} = 旧的 13 条，
与新增 16 id 交集为空。⇒ 语义变更在提交树内确为"潜在能力扩展、非行为变更"，实施方定性成立。**

> **复核过程中仓库状态发生了变化（非我所为）**：我开始复核时 `boot.rs` 为 `M`（未提交），
> 复核期间并行实施方将其提交为 **`53b965ae`（fix(core): LoadGFX 双实现归一，14:49:09）**。
> 因此下文对"提交树 d154544a vs 工作树"的二分，现应按 **d154544a（单点）vs HEAD=53b965ae（现状）** 来读。
> 我未对 53b965ae 做独立门禁复核（超出本次委托范围），但其消息中的关键自述与我的实测一致。

**但是（这条必须写进裁定）——HEAD 已经依赖该扩展：**

```
git status --short        → （复核开始时）M crates/game-core/src/frame/boot.rs
git diff HEAD -- boot.rs  →  - 删除 boot.rs 内第 4 份 load_gfx（usize 版简版副本）
                             + for id in [3u8, 4, 5, 6, 25, 22] { super::gfx::load_gfx(mem, id); }
现在：git log -1 → 53b965ae "LoadGFX 双实现归一——InitBank1 简版副本并入全形"
      git blame -L 287,288 → 53b965ae（已不再是 Not Committed Yet）
```

即 **HEAD 的 `init_bank1` 把 id 3（BrinBG1）与 id 6（BrinstarSprites）送进
`gfx_info`/`gfx_block`**——正是 16 个新增 id 中的两个。**若不先做 16-id 扩面，该调用会在
`unported!("bank07:C6E0-entry")` 直接 panic。**
⇒ 准确表述：**相对 d154544a 单点=潜在扩展（零行为变化成立）；相对 HEAD=已生效的必要前提
（且是 `init_bank1` 唯一的表读取路径）。** 实施方 commit 里「只是潜在能力扩展」的定性，
对 d154544a 单点**正确**，对现状**不完整**（且 53b965ae 自己也承认「当前矩阵不覆盖 InitBank1」）。

**asm 侧真相（回答"13 条 match 是不是真缺口"）**——`refs/m1disasm/SRC/prg7_engine.asm`：

```
:1523 InitBrinstarGFX: ldy #_id_GFX_BrinBG1(3) / CREBG1(4) / CREBG2(5) / _id_GFX_BrinstarSprites(6) /
                      Solid_0FC0(25) / jmp Solid_1FC0(22)
:1542 InitNorfairGFX : 4,5,7,8,9,25,22
:1564 InitTourianGFX / :1595 InitKraidGFX / :1620 InitRidleyGFX：使用 10,11,12,13,14,15,16,17,18,19 等
:1669 GFXInfo 表全 29 项（Entry 0..28，含 1 EndingSprites / 2 TheEndFont / 26 TourianFont / 28 ExclamationPoint）
```

⇒ **相对 asm，旧的 13 条 match 是"真缺口"**（至少 id 3、6 属 InitBrinstarGFX 必需，1/2/10-19/26/28 属
后续区域/Boss/结局例程必需）；只是这些例程在 Rust 侧要么尚未端口、要么（Brinstar）当时走了 boot.rs 自己的
第 4 份副本，所以缺口**没有在任何执行路径上暴露**。因此这次改动的性质是：
**"扩面消除相对 asm 的真缺口，且顺手把第 4 份实现收编进来"**——不是纯风格重构，实施方"机制折叠"的定位偏保守。

### 3.5 第 5 步：asm 锚点行号（亲自 grep）

```
1669:GFXInfo: ; 07:C6E0          ✓（GFXInfo 在 07:C6E0）
1748:LoadGFX: ; 07:C7AB          ✓
1786:CopyGFXBlock: ; 07:C7D5     ✓
宏体 macros.asm:101 `.macro GFXInfoEntry args gfx, dest` → byte bank / PtrTableEntry / word dest / word size ✓
```

三个锚点全部与声称一致。

### 3.6 事二小结

- 三份逐字节等价性：**全 True**（我独立解析源码数字 + 独立读 ROM，未采信实施方数字）。
- 29 项 ROM 直比：**True**；`sum(sizes)==GFX_DATA.len()==23712`：**True**。
- 13 块常量：**13/13 True**；旧 match 13 臂：**13/13 True**（追加项）。
- 语义变更定性：**提交树内零行为变化成立**（16 新 id 无生产调用点，实测）；**但工作树已依赖**；
  **相对 asm 是真缺口扩面**。
- `ROOM_PTR_TABLE` 删除：`git grep ROOM_PTR_TABLE d154544a^ -- crates/` 仅命中定义处 1 行，
  改后 0 命中 ⇒ **确为死代码，删除无风险**（我验证）。

---

## 四、问题清单

### 事一（`5ff70e3c`）

| # | 级别 | 问题 |
|---|---|---|
| A1 | **中** | `room.rs:1244` 注释与文档标题把 `(work_cycles, dropped)` 整体称为「emu 实录」；`work_cycles` 实为 Rust `LOAD_*` 模型累加量（`grep work_cycles` 无 emu 来源）。证据等级应为 **B（条件于 6 个未校准常量）**，非 A |
| A2 | **中** | room23（46456→2）、room8（71946→3）两点**无任何在库指针**（trace 只含房 0/9/19/20/240/241/243/244/255，无 23/8）；文档 §2 仅说"既有校准" |
| A3 | **中** | 「A 级」证据载体未入库：`analysis/traces/probe-loadwin.frames.jsonl` 未被 git 跟踪（`analysis/` 在 .gitignore:19,24），场景 `/tmp/probes/probe-loadwin.txt` 亦未入库。不可持久复现 |
| A4 | **中** | `docs/findings/load-window-multipoint-calibration.md:98`「28666c 的 work 就要 2 帧掉帧」数值错误（应为 ≥30560 @BASE=29000，或 31446 @区间下沿），且与本文件 §3 表 room20=31446 自相矛盾 |
| A5 | 低 | 5 点中仅 2 点参与夹逼（room20 下界、work=0 上界）；§3.1 只解释 room19/9 零贡献，未说明 room23/8 同样零贡献，「五点」易被误读 |
| A6 | 低 | work 值取证器（`[loadwin]` eprintln 探针）取证后 `git checkout` 回退，未留可复跑脚本 → x 坐标不可重算 |
| A7 | 低 | room9 点取自与 emu 错位 28 帧的运行（文档 §2.3 自标 ❌），仍作为校准输入使用，未附置信折扣（对交集无影响） |

### 事二（`d154544a`）

| # | 级别 | 问题 |
|---|---|---|
| B1 | 低（已被 53b965ae 补） | commit message「此前被实现了**三遍**」**漏计第 4 份**：`d154544a^:crates/game-core/src/frame/boot.rs` 的 `pub fn load_gfx(mem: &mut FrameMemory, id: usize)` 是独立**简版且语义不同**的副本（缺 BankSwitch 双切/描述子倒序拷/`$05/$06` 归零）。三遍→**四遍**。该遗漏已由 `53b965ae`（14:49）修正，其消息同样定位为「第四份重复」 |
| B2 | **中** | 「零行为变化」的 PPU 证据用 `case-password-justin.sched`。无论该用例是否触发 `init_bank1`（`boot.rs:252` 在 bank-init 派发表 index=1），**提交时点的 Brinstar 载入走的是 `boot.rs` 自己的第 4 份 `load_gfx`（usize 版），根本不经过 `frame::gfx::gfx_info/gfx_block`** ⇒ 该 PPU 证据对 id 3/6 经新实现的首载**零覆盖**，与工作树补件改动的正是这条路径不重合 |
| B3 | **中** | HEAD（`53b965ae`）的 `init_bank1` 把 id 3/6 送入本函数，**依赖本 commit 的 16-id 扩面才不 panic**；但 d154544a 的"潜在扩展/零行为变化"表述未提此事（当时该改动尚在未提交工作树，我实测 `git blame`=Not Committed Yet，复核期间被并行实施方提交为 53b965ae）→ 只读 d154544a 会误判扩面为空转；须与 53b965ae 合读 |
| B4 | 低 | `gfx_id` 常量模块仍只有 13 个 id，**缺 `BRIN_BG1 = 3`、`BRIN_SPRITES = 6`**；而这些 id 现已被 `init_bank1` 使用（工作树）。"单一真源"在常量面仍未补齐，仍是"magic number 与常量模块两份" |
| B5 | 低 | `gfx_block` 越界保护：生产 `id>=29` 走 `unported!`（panic，安全）；内部仅 `debug_assert!(off+size<=len)`，release 靠切片越界 panic 兜底——可接受，但不是显式校验（生成期校验依赖 bake_gfx.py） |
| B6 | 低 | 提交内混入 `mainloop.rs` 删除 `ROOM_PTR_TABLE`（我验证确为死代码，仅定义处 1 行）——按 AGENTS §9「小步提交：一个 commit 一个逻辑变更」，属混提（影响可忽略） |

---

## 五、置信度

- 事一（算术与区间）：**97**（纯整数运算，我独立重解 + 逐点回代，无歧义）
- 事一（证据等级判定）：**88**（`work_cycles` 非 emu 源这一点有 grep 硬证；「应为 B 级」的评级含判断成分）
- 事二（三份等价性）：**99**（独立解析源码 + 独立读 ROM，四次全量比对全 True，可一行复跑）
- 事二（语义变更定性）：**93**（调用图 grep 穷尽；工作树补件为实测 diff + blame；仅"是否有其它动态 id 路径"存在极小残余）
- 综合：**94**

## 六、复现命令（全部只读）

```bash
# 事一：区间重算（不依赖仓库断言）
python3 - <<'PY'
F=29780; P=[(31446,2),(36620,2),(43378,2),(46456,2),(71946,3)]
lo=max(d*F-w for w,d in P+[(0,0)]); hi=min((d+1)*F-w for w,d in P+[(0,0)])
print(lo,hi,hi-lo,(lo+hi)/2,29000-(lo+hi)/2)      # 28114 29780 1666 28947.0 53.0
PY

# 事一：从在库 trace 复算掉帧窗（需本地保留的 analysis/traces/probe-loadwin.frames.jsonl）
python3 - <<'PY'
import json
prev=None; z=[]
for l in open('analysis/traces/probe-loadwin.frames.jsonl'):
    o=json.loads(l); fc=o['w'].get('FC')
    if prev is not None and fc==prev: z.append(o['f'])
    prev=fc
print(z)   # [2,4,5,6,311,312,313,352,...,358,896,897,1068,1069,1287,1288]
PY

# 事一：唯一发现的数据错误
sed -n '98p' docs/findings/load-window-multipoint-calibration.md

# 事二：三份等价性（见 §3.1-3.3 脚本，输出应为 True / 29-29 True / 13-13 True）
# 事二：第 4 份副本与工作树依赖
git show d154544a^:crates/game-core/src/frame/boot.rs | grep -n "pub fn load_gfx(mem: &mut FrameMemory, id: usize)"
git diff HEAD -- crates/game-core/src/frame/boot.rs | head -40
git blame -L 287,288 --date=short crates/game-core/src/frame/boot.rs

# 事二：asm 锚点
grep -n "GFXInfo: ; 07:C6E0\|LoadGFX: ; 07:C7AB\|CopyGFXBlock: ; 07:C7D5" refs/m1disasm/SRC/prg7_engine.asm
```
