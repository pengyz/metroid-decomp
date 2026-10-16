# spec：state-bundle 状态快照夹具（场景级任意起点，task-32）

> 状态：v1（2026-10-16）——装载能力已定谳
> （[savestate-load-capability.md](../findings/savestate-load-capability.md)，A 级矩阵）；
> fixture DSL 实战补账已过（三条件验证，见 [fixture 实证](#3-实战补账已验)）。
> **game-core 恢复面（MSS↔FrameMemory 映射）=porter 分工**（其 10 态肉后接）——
> 本文即其分工件详案的规格基础。

## 1. bundle schema（v1）

```json
{
  "schema": 1,
  "id": "<bundle-id>",
  "scenario": "<scenario 名>",
  "anchor": { "frame": <F_a>, "scenario_sha256": "<锚>" },
  "snapshot": {
    "format": "MSS",
    "sha256": "<blob 摘要>",
    "source": "emu.createSavestate()（exec 内存回调上下文）",
    "note": "全机状态含 RAM（A 级实证：标记恢复往返）"
  },
  "fixture": [
    { "addr": "<hex>", "value": "<hex>", "persistence": "invariant|evolving",
      "evidence": "<A/B 级证据指针>" }
  ],
  "verify": {
    "k_max": 120,
    "conditions": ["a_state_eq", "b_write_readback", "c_persistence"]
  }
}
```

- **snapshot**：MSS blob（`emu.createSavestate()` 返回值）——存档级对拍基座；
  快照字节入库判据同夹具（可再生产生则只存 sha256+复现命令）；
- **fixture**：叠加写集（设计 state-fixture §3 F_a 注入模型——每址 persistence+
  证据指针）；
- **verify**：三条件（设计 §3：a=声明域内 F_a+1 全等 / b=写入地址读回 / c=invariant
  窗口末持久）。

## 2. 装载流（oracle 腿，能力已证）

```
exec 内存回调点（fixture 触发路径——能力约束实录）
  ├─ loadSavestate(snapshot.blob)      # 全机恢复（RAM 含）✓ 已证
  ├─ fixture 叠写（emu.write, MEMTYPE=8）  # ✓ 实战已证
  └─ 采样续行（spin-exit 面同址）        # ✓ 执行连续性已证
```

**约束（A 级实录）**：
1. createSavestate/loadSavestate **必须在主 CPU exec 内存回调内**（错误串原文见
   findings §1）——触发点=exec 回调（与 spin-exit 采样同机制）；
2. `getState/setState`=**寄存器级**（不含 RAM）——bundle 的 RAM 面只经 MSS blob；
3. MEMTYPE=8（数值枚举）；无参 createSavestate 阻塞（禁用）。

## 3. 实战补账（已验）

fixture DSL（`fixture <帧> <addr> <值> [invariant|evolving]`）实测
（scenario `oracle/scenarios/fixture-test.txt`，复跑 `scenario_run run fixture-test`）：

```
fixture applied f=200 $0700=$ab (invariant) ok=true
(b) F_a+1 读回: f201 $0700 = AB  $0701 = CD        ← 期望 ab/cd ✓
(c) invariant 窗口末 f299 $0700 = AB（保持）✓      evolving $0701=CD（如实）
(前) f199 $0700 = 00（注入前）✓
run.json fixtures 记录 ✓
```

## 4. game-core 恢复面分工件（porter——MSS↔FrameMemory 映射）

**目标**：双腿任意起点（bundle 装载后 game-core 腿与 oracle 腿同态起跑）。
**接缝规格**：

| MSS 面（源） | FrameMemory 面（目标） | 备注 |
|---|---|---|
| RAM 区（$0000-$07FF） | `ram[0x800]` | 直映（A 级） |
| WRAM（$6000-$7FFF） | `wram[0x2000]` | 直映 |
| CPU 寄存器（cpu.a/x/y/sp/pc/ps） | `regs.{a,x,y,sp,pc,ps}` | 直映 |
| $1A NMIStatus/$2D FrameCount 等 RAM 位 | ram 内 | 直映（含在 RAM 区） |
| mapper.prgReg/chrReg/prgMode | **memmap 扩展面** | bank 切换态——task-14 A9 后遗同面 |
| ppu.*（scanline/vram/scroll…） | **S6 渲染腿需求** | bundle v1 可不含（L3 状态面无 PPU） |
| apu.* / clock / frameCount | **时钟/RNG 对表** | GameRng 状态映射待 RNG 定谳（task-10 序） |

**验收**（bundle 装载对拍）：bundle 装载后 game-core `watch_snapshot` == oracle
F_a+1 采样（声明域内逐字节）——即设计三条件 (a) 的双腿形态。

## 5. 遗留分工表

| 件 | 归属 | 状态 |
|---|---|---|
| fixture DSL + F_a 注入 + run.json 记录 | decomp | **已落**（本文 §3） |
| bundle schema（本文） | decomp | **已落** |
| MSS↔FrameMemory 映射实现 | **porter**（10 态肉后接） | 待（本文 §4 规格） |
| bundle 文件格式落库判据（sha256 vs 存 blob） | lead 裁定 | 待 |
| diff_traces 三条件断言消费（fixture manifest） | decomp（随首 bundle 场景） | 待 |
| PPU 面 bundle 扩展 | S6 | 远期 |
