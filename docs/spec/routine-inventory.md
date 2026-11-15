# 例程清单（结构驱动迁移的分母）

> 目的：把「还缺什么」从「L4 哪一格像素不对就挖哪格」变成**机械可查的清单**。
> 本文件只记录口径与当前读数；数字由 `tools/routine_inventory.py` 生成，
> 门禁 `routine-inventory` 保证它只减不增。

## 1. 为什么需要这份清单

在清单出现之前，覆盖增长靠偶然：L4 用例走到哪里，就覆盖到哪里。代价是实测的：

- 「110/122 格全一致」在把输入窗移出开场演出后，立刻暴出 **22 格真实分歧**
  （之前的一致是两侧都不动）；
- 敌出生 `data05` 的一处补偿赋值（`status & ScrollDir`）在横滚房恰好等价于「朝左」，
  看起来对；按 asm 改正反而令 3 格由 match 变 diff，因为真正的写入者（x 轴力）没端口；
- 真实分母与「看起来完成」差得远：敌人 AI 只接 2/16 种、区域只进过 1/5、
  萨姆斯 10 态里只有 4 态有状态级证据、声音 handler 只触达 4/28。

这类问题的共同点是：**没有机械分母**。本清单就是给行为面造一个。

## 2. 分母从哪来

- **不使用** `refs/m1disasm/out/M1_NES_NTSC.sym` 的 per-bank 实例计数。同一个例程会被
  5 个区域 bank 各自 `include` 一次，按实例数会把分母放大 5 倍，且每份实例没有独立信息量。
- **使用** `refs/m1disasm/SRC/**/*.asm` 的**源码级 label**，并按「下一条非空行是不是数据指示
  （`.byte`/`.word`/`.dw`/`.ds`/`.incbin`…）」分成 `code` 与 `data` 两类：
  - label 允许 `_{AREA}` 这类宏占位（`SkreeAIRoutine_{AREA}:`），登记时**去掉占位后缀**，
  否则 `enemies/*.asm`（敌 AI 全部用 `{AREA}` 参数化）会整批漏掉；
- 分类要穿透两种情况：①**label 链**（多个标签指向同一块数据）②`@loop` 这类局部标签；
  穿透后若首 token 不是 6502 助记符而是 `SetPaletteEntry`/`SignMagSpeed` 这类**生成数据的宏**，
  仍按 data 计（否则 `palettes.asm` / `enemy_sprite_data` 会被当成上千条例程）；
- `SRC/songs/pal/` 整棵排除（D6：保真期只做 NES_NTSC）；
- 汇编器自动名不算例程：`L####`（wla-dx 自动跳转标）与 `RTS_*`（局部出口标）整类剔除，
  否则一个例程里每处跳转目标都会被当成「一条待移植例程」（实测虚增 244 条）；
- overrides 支持 `status: "data"`：分析器被 `.if`/宏包住的表判成 code 时，可显式退回
  数据面（不计数、不算缺口），必须写 note 说明是哪种表。
- `code` = 例程 → **本清单的分子分母都只算它**；
  - `data` = 数据表 → 已由资产包 `pack-tool verify` 逐段对 ROM 自证，这里只登记、不计缺口。

当前分母：**code 1220 条 / data label 2625 条**（另有 `@loop` 类局部标签与
`L####`/`RTS_####` 自动名不计）。

## 3. 三档状态（口径）

| 状态 | 判据 | 可信度 |
|---|---|---|
| `ported` | 在 `data/status/routine_inventory_overrides.json` 里**显式登记了 Rust 符号** | 人工/考证映射，最可信 |
| `candidate` | label 的 snake_case 与某个 Rust `fn` 同名 | **线索，不是结论**：同名不等于行为等价 |
| `unported` | 两者都没有 | 缺口 |

`candidate` 必须按对应 asm 例程逐行复核（输入/输出/副作用/边界）后才能升 `ported`；
只靠名字相同就写「已端口」是本项目已经栽过跟头的做法（见 `docs/knowledge/` 里
「断言自实现」那条 gotcha）。

## 4. 当前读数

```
code 例程 1220：ported 20 / candidate 228（18.7%）/ unported 972
data label 2625（不计缺口）
```

按文件分组（非 ported = candidate + unported）：

| 来源 | code | candidate | unported |
|---|---|---|---|
| `prg7_engine.asm` | 918 | 168 | 750 |
| `prg0_title.asm` | 251 | 23 | 228 |
| `sound_engine.asm` | 151 | 37 | 114 |
| `areas_common.asm` | 87 | 10 | 77 |
| `prg3_tourian.asm` | 74 | 2 | 72 |
| `common_sprite_data.asm` | 44 | 0 | 44 |
| `prg6_graphics.asm` | 19 | 0 | 19 |
| `prg1/2/4/5` 区域入口 | 69 | 4 | 65 |
| `enemies/*.asm`（16 文件） | 40 | 0 | 40 |

> 注：`candidate` 偏低（14.2%）不是坏消息，是**诚实**——Rust 侧大量例程用了
> 与 asm 不同的命名（如 `WaitNMIPass → iter_wait`），名字匹配天然抓不到；
> 这些要靠 `overrides` 逐条登记，正是接下来要做的活。

## 5. 缺口排序（按「挡住可玩性」的程度）

1. **敌人 AI**（`enemies/*.asm` 16 文件 + `areas_common.asm` 的 `Enemy*` 27 条 +
   `prg7` 18 条）：只接了 crawler / skree 两种。这是 24 个残差格里最集中的一类，
   也是「游戏不像原版」的最大单一来源。
2. **声音**（`sound_engine.asm` 114 条非端口）：28 个 handler 只有 4 个触达；
   后果是音乐/SFX 只在矩阵走过的那条窄路上正确。
3. **NMI / PPU / VRAM 写入面**（`prg7` 34 + `title` 65）：既有 `#8`（`fc` 相位 +2、
   `SetPPUInc` 未建模）就挂在这里。
4. **萨姆斯动作与状态**（`prg7` 22）：炸弹（`CheckBombLaunch`）未实现、死亡链薄、
   变球/上瞄零证据。
5. **区加载 / 门 / 电梯**（`Room` 27 + `Door` 19）：前三条换区边与结局径无像素证据。
6. **数学与工具例程**（`Addiv/Amul/Base10/…` 13 条）：被上面各档依赖，风险低但
   一旦错会污染整条链。

排序依据不是「代码量」，而是**它挡住多少条已有用例 / 多少个未验证状态**。
每一项的验收方式固定三段：单测（语义）→ 逐帧状态（电解剖）→ 像素格（L4）。

## 6. 如何维护

```bash
python3 tools/routine_inventory.py --stats            # 看读数
python3 tools/routine_inventory.py --list unported --group prg7_engine --limit 60
python3 tools/routine_inventory.py --write            # 重生成（改了 asm 源码或 overrides 后）
python3 gates/cli.py check                            # 门禁：unported 不得增长
```

- 复核完一条例程后，在 `data/status/routine_inventory_overrides.json` 的 `labels` 里
  登记 `{"<Label>": {"status": "ported", "rust": "<path>:<line>"}}`，再 `--write`；
- 门禁三条判据：①`unported` 数只减不增 ②`refs/m1disasm/SRC` 指纹与清单锚一致
  ③没有清单外新例程。改了 asm 证据（`.sym`/`SRC`）必须显式重生成并在 commit 说明。

## 7. 这条线的边界（诚实声明）

- 清单是**行为面的声明性分母**，不是机械完备性证明（AGENTS.md D7：行为空间无机械分母）。
- `unported` 里有 ROM 死代码与不可达桩，不是每一条都该实现；登记为
  `out-of-scope` 需要证据（不可达性论证），不能靠感觉抹掉。
- 清单产出「还缺什么」；**对不对仍然只由 L4 判**（零容差，不修订）。
