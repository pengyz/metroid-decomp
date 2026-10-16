# findings：Mesen2 savestate 装载能力探针（task-32 首件，state-bundle 前置）

> 状态：**装载面能力定谳**（2026-10-16，v1-v4 探针迭代）。m3-move 时代 savestate.core
> 存档面已证；本件=**装载面**实测（场景级任意起点的前置能力）。
> 复跑：`xvfb-run -a tools/mesen2/Mesen "rom/Metroid (USA).nes"
> tools/probe_savestate_load.lua /DoNotSaveSettings` + 读
> `analysis/probe_savestate_load.log`。

## 1. 能力矩阵（A 级，输出原文摘录）

| API | 能力 | 实证 |
|---|---|---|
| `emu.createSavestate()` | **返回 MSS 格式二进制 blob**（全机状态含 RAM） | ret_type=string，头 `MSS…`；**须 exec 内存回调上下文**（错误串：「This function must be called inside an exec memory operation callback for the main CPU」） |
| `emu.loadSavestate(blob)` | **全机恢复**（RAM 恢复实证）+执行续行 | 「exec 内覆写: 标记=$33」→loadSavestate ok=true→「装载后: 标记=$55（快照值）」→「f35 连续性: 标记=$55」 |
| `emu.getState()/setState()` | **寄存器级状态表**（CPU/APU/PPU/mapper.prgReg/frameCount…）——**不含 RAM** | 键族全枚举（无 ram/iram 面）；setState ok=true 但 RAM 标记不恢复（$55 不动） |
| ~~emu.saveStateFile/loadStateSlot 等~~ | **不存在** | v1 猜名全错（枚举面救场=r3 教训同款：API 名必须枚举实证） |

## 2. 关键约束（B 级推断→设计输入）

1. **上下文约束**：createSavestate/loadSavestate **必须在主 CPU exec 内存回调内调用**
   ——fixture 触发路径=exec 回调点（与 spin-exit 采样面同址机制）；
2. **MEMTYPE=8 数值枚举**（`emu.memType.cpu`=nil 实录——scenario_capture 同源口径）；
3. **无参 createSavestate 阻塞**（v2 超时 124 实录，疑原生对话框）——禁用无参形态；
4. setState 面=寄存器快照（跨进程边界仅此可序列化为表格）；**RAM 面=MSS blob 专属**。

## 3. state-bundle schema 基础（设计输入，bundle schema 待落）

```
bundle = {
  snapshot: MSS blob（createSavestate——全机含 RAM；存档级对拍基座）,
  fixture:  [{addr, value, persistence}]（fixture DSL 写集——叠加注入，设计 §3 F_a）,
  anchor:   {frame, scenario_sha256}（场景锚）
}
```
- 装载流：exec 回调点 loadSavestate(blob) → fixture 叠写 → 断言面（三条件同源证明）；
- game-core 恢复面（FrameMemory 反序列化）=porter 侧（其 10 态肉后接）——
  **MSS blob↔FrameMemory 映射表**是双腿任意起点的最后接缝。

## 4. 分级与遗留

| 结论 | 级 |
|---|---|
| API 能力矩阵+往返实证 | **A**（探针输出原文） |
| 上下文约束/MEMTYPE | **A**（错误串+实录） |
| bundle schema 雏形 | **B**（设计输入） |
| MSS blob 格式解析（逐段结构） | **C**（待考） |

遗留：①fixture DSL 实战补账（fixture 指令×exec 回调触发路径合龙）②bundle schema
落 spec ③MSS↔FrameMemory 映射（porter 分工）④无参阻塞形态的根因（原生对话框假说
——可 xvfb 下 xdotool 复核，低优先）。
