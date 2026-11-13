# LoadGFX（07:C7AB）在 rust 侧存在两份语义不同的实现

**日期**：2026-11-13
**触发**：止血件 3（`d154544a`，GFX 烘焙去重）复核时发现「三份重复」之外还有第四份，且它并非等价副本。
**结论级别**：A（asm 逐行 + ROM/表逐字节 + 两处 rust 源码对读）

## 1. 现象

同一个 asm 例程 `LoadGFX: ; 07:C7AB`（refs/m1disasm/SRC/prg7_engine.asm，含 `CopyGFXBlock: ; 07:C7D5`）在 rust 侧有两份实现：

| 位置 | 签名 | 调用者 |
|---|---|---|
| `crates/game-core/src/frame/gfx.rs:121` | `pub fn load_gfx(mem: &mut FrameMemory, id: u8)` | `init_title_gfx` / `init_norfair_gfx` / `load_samus_gfx`（即 InitBank0 与 InitBank2 路径） |
| `crates/game-core/src/frame/boot.rs:219`（已删） | `pub fn load_gfx(mem: &mut FrameMemory, id: usize)` | `init_bank1`（InitBank1 = **Brinstar 游戏入场链**）及其两条探针测试 |

## 2. 两者语义差异（不是等价副本）

`frame/gfx.rs` 版按 asm 全形执行四段：

1. **描述子倒序拷贝** → `$00-$06`（asm `LC7B6: x=6..0 ← GFXInfo[7*id + 6..0]`）
2. **BankSwitch 进切** → `$00=gfx_bank`、`$28=($28&0x18)|gfx_bank`、MMC1 PRG 串行写（asm `jsr BankSwitch`）
3. `$FF &= ~INCR_DOWN` → CopyGFXBlock `$2006/$2007` 走带
4. **`$05/$06` 归零**（asm `dec Temp05_GFXSize` 消耗环终态）+ **回切 CurrentBank**（asm `ldy CurrentBank / jmp BankSwitch`）

`boot.rs` 旧版只做了 3 与「`$FF` 清位」：**缺 1、缺 2、缺 4**。也就是说 Brinstar 入场链走的是「不换 bank、不拷描述子、不回切」的简版。

## 3. 为什么这是缺陷而不是等价折叠

- asm 只有一份 `LoadGFX`，任何调用者（InitTitleGFX / InitBrinstarGFX / InitNorfairGFX / LoadSamusGFX）都走同一例程、都执行 BankSwitch 双切。rust 侧让 Brinstar 路径绕开这些副作用，等于**在原版会写 RAM 的地方不写**：`$00`（Temp00_Bank）、`$28`（SwitchUpperBits）、`$05/$06`（GFXSize 终态）在 InitBank1 之后处于与原版不同的值。
- 这与止血件 3 修掉的「同一份 GFXInfo 表烘三遍」属同一类问题（机制在 rust 侧被复制且漂移），但危害更大：前者只是数据冗余（值等价，可机械校验），后者是**行为分叉**（不可由数据比对发现）。
- 它此前未被 L4 抓到，原因是当前 14 个用例都不走 InitBank1（Brinstar 常规入场）——`init_bank1` 只在 BankTable 索引 1 时被 `c4de_unit` 分派。

## 4. asm 依据（A 级）

- `GFXInfo: ; 07:C6E0`（prg7_engine.asm:1669 起）：29 项 `GFXInfoEntry`，注释自带 Entry 序号；Entry 3 = `GFX_BrinBG1`、Entry 6 = `GFX_BrinstarSprites`。
- `InitBrinstarGFX: ; 07:C604`（:1523）：`ldy #_id_GFX_BrinBG1` → CREBG1 → CREBG2 → `_id_GFX_BrinstarSprites` → Solid_0FC0 → `_id_GFX_Solid_1FC0 / jmp LoadGFX`，即 id **3/4/5/6/25/22**。
- `LoadGFX: ; 07:C7AB`：描述子 7B 倒序 → `jsr BankSwitch` → `$FF &= ~INCR_DOWN` → `jsr CopyGFXBlock` → `ldy CurrentBank / jmp BankSwitch`。

## 5. 修复

**commit**：见同批 `fix(core): LoadGFX 双实现归一`。

- 删除 `crates/game-core/src/frame/boot.rs` 内的简版 `load_gfx(usize)`。
- `init_bank1` 的装载循环改调 `super::gfx::load_gfx(mem, id as u8)`（id 集仍为 asm 的 3/4/5/6/25/22，不变）。
- 两条探针测试（`crebg2_covers_1f00` / `samus_gfx_tile09`）改调统一实现。

**验证**：

- `cargo test --workspace` → 538 passed / 0 failed。
- 单案 `rust_screenshot --schedule shots_rust/case-password-justin.sched` 与既有产物比对：**14/14 ppu.hex 逐字节相同**（当前矩阵不覆盖 InitBank1，故零像素变化符合预期）。
- 归一后的行为差异只出现在 InitBank1 路径上（`$00/$28/$05/$06` 与 BankSwitch 双切），方向是**更贴 asm**。

## 6. 遗留

- **InitBank1（Brinstar 常规入场）无 L4 用例覆盖**——这是第 3 节点出的「当前矩阵不覆盖」的直接后果。要真正验证这次归一，需要一个走「标题 → START → 常规新游戏 → Brinstar 首房」的用例（现有 `case-password-justin` 走密码进场 = Norfair，`case-001` 等不走 InitBank1 分派）。建议立卡：`case-brinstar-entry`。
- 同族待查：是否还有其它 asm 例程在 rust 侧存在「简版 + 全形」两份（本次是 `LoadGFX`；`BankSwitch` 有 `bank_switch`/`c4de_unit` 内联两处，需单独核）。
