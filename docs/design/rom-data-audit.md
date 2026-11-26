# ROM 数据硬编码审计与迁移计划（crates 常量清单 → game-data 导出）

> 只读审计，不改代码。本文所有数字可由 §9 的命令复算。
> 原则（Human 定）：**Rust 里不包含 ROM 数据，ROM 数据由导出工具产出**。

## 0. 结论

- 全仓 `const [X; N>=10]` 命中 **22 条**（§1 清单，一条不漏）。
- 其中 **13 条是 ROM 数据**（12 张表，`BANK_TABLE` 在 `title/bridge.rs` 与 `frame/tick.rs` 各一份，字节相同）：每条都能给出 `bank:addr` 与 ROM 字节匹配（§2，A 级）。
- **6 条是派生常量/硬件规格表**（APU 三张、JB 字符码、重生动画序、全 FF 填充）：ROM 里没有对应字节序列，按公式生成（§4）。
- **3 条不是 ROM 数据**（Rust 侧地址清单、两处测试夹具）（§5）。
- **一处超读**：`BANK_TABLE: [u8; 16]` 的 ROM 真表只有 **5 B**（`BankTable ; 07:CA30`），后 11 B 是紧随其后的“Saved game routines (not used)”代码。消费方只用前 5 项，行为未受影响，但按「表」烘焙会把代码当数据（§3）。
- `include_bytes!` / `include_str!`：**0 命中**。读 `rom/`：`grep -rn 'rom/Metroid' crates/ --include=*.rs` **15 命中**，其中**生产路径 0 处**，唯一**代码级**读取是 `crates/game-data/src/extract.rs:1957`（`#[cfg(test)] mod tests` 的 `area_banks_derive_from_rom_bank_table`，无 ROM 时 `eprintln` 跳过），其余 14 条为注释/用法示例/提示字符串（§1.2）。
- 迁移分 3 批（§7）：低风险 **6 张表 / 7 条常量**（标题/HUD 静态表，`BANK_TABLE` 两份）→ 中风险 5 张（区域敌人表）→ 高风险 1 张（`MAIN_ROUTINE_PTR_TABLE`，逐帧热路径）。合计 12 张表 / 13 条常量。

## 1. 扫描清单

### 1.1 命令

```bash
grep -rnE 'const [A-Z0-9_]+: *\[(u8|u16|i8|i16); *[0-9]{2,}\]' crates/
```

命中 22 条，逐条列出：

| # | 文件:行 | 常量 | 长度 | 分类 | ROM | 证据 |
|---|---|---|---|---|---|---|
| 1 | `crates/game-core/src/ui/hud.rs:10` | `DATA_DISPLAY_TBL` | 40 B | ① ROM | `07:E1B9` | A |
| 2 | `crates/game-core/src/ui/password.rs:105` | `NAR_PASSWORD_TBL` | 16 B | ① ROM | `00:8E07` | A |
| 3 | `crates/game-core/src/title/mod.rs:1117` | `PAL_TBL` | 16 B | ① ROM | `00:9B83` | A |
| 4 | `crates/game-core/src/title/mod.rs:1557` | `PW_NOERR_MSG` | 15 B | ② 派生（`[0xFF;15]` 填充） | — | A（ROM 无此序列） |
| 5 | `crates/game-core/src/title/bridge.rs:15` | `TITLE_ROUTINE_PTR_TABLE` | 31×u16 | ① ROM | `00:802A` | A |
| 6 | `crates/game-core/src/title/bridge.rs:497` | `PASS_WORD_VRAM` | 21 B | ① ROM | `00:937F` | A |
| 7 | `crates/game-core/src/title/bridge.rs:584` | `JB_CHARS` | 24 B | ② 派生（密码字码+空格填充） | — | A（ROM 无此序列） |
| 8 | `crates/game-core/src/title/bridge.rs:728` | `BANK_TABLE` | 16 B | ① ROM（**超读**：真表 5 B） | `07:CA30` | A |
| 9 | `crates/game-core/src/title/bridge.rs:846` | `ITEM_DATA` | 59×u16 | ① ROM | `00:9029` | A |
| 10 | `crates/game-core/src/world/tile_blast.rs:57` | `RESPAWN_ANIM_INDEX` | 10 B | ② 派生（重生动画序） | — | A（ROM 无此序列） |
| 11 | `crates/game-core/src/samus/state.rs:334` | `TYPED_ADDRS` | 23×u16 | ③ 结构（Rust 侧地址清单） | — | — |
| 12 | `crates/game-core/src/frame/tests.rs:1056` | `T` | 40 B | ③ 测试夹具 | — | — |
| 13 | `crates/game-core/src/frame/tick.rs:36` | `MAIN_ROUTINE_PTR_TABLE` | 10×u16 | ① ROM | `07:C141` | A |
| 14 | `crates/game-core/src/frame/tick.rs:798` | `BANK_TABLE` | 16 B | ① ROM（**超读**：真表 5 B） | `07:CA30` | A |
| 15 | `crates/game-core/src/enemy/kraid.rs:68` | `PROJECTILE_OFFSET_X` | 10×i8 | ① ROM | `01:9CBD`（另 `04:9BD1` 同字节） | A |
| 16 | `crates/game-core/src/enemy/metroid.rs:50` | `METROID_LATCH_OFFSET_Y` | 12×i8 | ① ROM | `03:99D8` | A |
| 17 | `crates/game-core/src/enemy/cannon.rs:22` | `INSTR_LISTS` | 45 B | ① ROM | `03:9D99` | A |
| 18 | `crates/game-core/src/enemy/state.rs:888` | `TBL` | 18 B | ③ 测试夹具（`mod tests` 内） | — | — |
| 19 | `crates/game-core/src/enemy/rinka.rs:309` | `SPAWN_POS_TBL` | 12 B | ① ROM | `03:A1DB` | A |
| 20 | `crates/game-platform/src/apu.rs:27` | `LENGTH_TABLE` | 32 B | ② 硬件规格表 | — | A（ROM 无此序列） |
| 21 | `crates/game-platform/src/apu.rs:243` | `TRI_SEQUENCE` | 32 B | ② 硬件规格表（0..15 往返） | — | A（公式生成） |
| 22 | `crates/game-platform/src/apu.rs:249` | `NOISE_PERIODS` | 16×u16 | ② 硬件规格表（NES 噪声周期） | — | A（公式生成） |

`ui/password.rs:207` 的用法是「与输入逐字节比较」，`bridge.rs:595` 是「JB 密码写入密码格」——两处都只读常量、不写业务 RAM，迁移后行为不变。

### 1.2 `include_bytes!` / `include_str!` / 读 `rom/`

```bash
grep -rn 'include_bytes!\|include_str!' crates/            # 0 命中
grep -rn 'rom/Metroid' crates/ --include=*.rs              # 15 命中
```

15 条命中逐条定性（文件:行 → 性质）：

| 文件:行 | 性质 |
|---|---|
| `crates/game-data/src/extract.rs:1957` | **代码级读取**：`std::fs::read(concat!(CARGO_MANIFEST_DIR, "/../../rom/Metroid (USA).nes"))`，在 `#[cfg(test)] mod tests`（`:1889` 起）的 `area_banks_derive_from_rom_bank_table`；无 ROM 时 `eprintln!("跳过：无 ROM")` 并 `return` |
| `crates/game-data/src/pack.rs:995`、`:1001` | 注释：pack 头的重生成命令 |
| `crates/game-core/src/bin/rust_screenshot.rs:383` | `eprintln!` 提示字符串（缺 pack 时告诉用户去跑 `pack-tool`） |
| `crates/game-data/src/bin/game-data-inspect.rs:4` | 用法示例注释 |
| `crates/game-data/src/lib.rs:11` | ROM 身份注释（sha256 锚） |
| `crates/game-data/src/extract.rs:11` | ROM 身份注释 |
| `crates/game-data/src/area/graphics.rs:5` | `//!` 注释里的自证命令 |
| `crates/game-data/src/title/world_map.rs:5` | 交叉核对注释 |
| `crates/game-core/src/title/bridge.rs:496` | 交叉核对注释 |
| `crates/game-core/src/enemy/polyp.rs:88` | 交叉核对注释 |
| `crates/game-core/src/enemy/dragon.rs:84` | 交叉核对注释 |
| `crates/game-shell/game-shell-pc/src/main.rs:33` | `eprintln!` 提示字符串（缺 pack 提示；本卡范围外但全仓扫描列出） |
| `crates/pack-tool/src/main.rs:5`、`:6` | 模块注释里的用法示例 |

**结论**：生产路径无 `open("rom/...")`；唯一代码级读取在 `#[cfg(test)]` 且缺 ROM 时跳过，不构成「运行时依赖 ROM」。（本条为复核退回项，原稿写「12 命中、全部是注释」，漏了 `extract.rs:1957` 与两条 eprintln/一条 `//!`。）

## 2. A 级：13 条 ROM 数据的字节级验算

### 2.1 命令（一次算完 22 条）

```bash
python3 - <<'PY'
import re,pathlib
prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024]
def arr(path,name):
    s=pathlib.Path(path).read_text()
    m=re.search(r'const %s: *\[(u8|u16|i8|i16); *(\d+)\] *=\s*\[(.*?)\];'%name,s,re.S)
    typ,n=m.group(1),int(m.group(2)); body=re.sub(r'//[^\n]*','',m.group(3))
    vals=[int(t,0) for t in (x.strip() for x in body.split(',')) if t]
    if typ in('u8','i8'): return bytes(v&0xFF for v in vals)
    out=bytearray()
    for v in vals: v&=0xFFFF; out+=bytes((v&0xFF,(v>>8)&0xFF))
    return bytes(out)
C=[('crates/game-core/src/ui/hud.rs','DATA_DISPLAY_TBL','u8'),
   ('crates/game-core/src/ui/password.rs','NAR_PASSWORD_TBL','u8'),
   ('crates/game-core/src/title/mod.rs','PAL_TBL','u8'),
   ('crates/game-core/src/title/bridge.rs','TITLE_ROUTINE_PTR_TABLE','u16'),
   ('crates/game-core/src/title/bridge.rs','PASS_WORD_VRAM','u8'),
   ('crates/game-core/src/title/bridge.rs','BANK_TABLE','u8'),
   ('crates/game-core/src/title/bridge.rs','ITEM_DATA','u16'),
   ('crates/game-core/src/frame/tick.rs','MAIN_ROUTINE_PTR_TABLE','u16'),
   ('crates/game-core/src/enemy/kraid.rs','PROJECTILE_OFFSET_X','i8'),
   ('crates/game-core/src/enemy/metroid.rs','METROID_LATCH_OFFSET_Y','i8'),
   ('crates/game-core/src/enemy/cannon.rs','INSTR_LISTS','u8'),
   ('crates/game-core/src/enemy/rinka.rs','SPAWN_POS_TBL','u8')]
for p,n,t in C:
    b=arr(p,n); i=prg.find(b)
    if i<0: print(f'{n:24s} no-ROM-match'); continue
    bank=i//0x4000; addr=(0xC000 if bank==7 else 0x8000)+i%0x4000
    print(f'{n:24s} len={len(b):3d} ROM {bank:02X}:{addr:04X} occ={prg.count(b)}')
PY
```

### 2.2 实跑输出

```
DATA_DISPLAY_TBL         len= 40 ROM 07:E1B9 occ=1
NAR_PASSWORD_TBL         len= 16 ROM 00:8E07 occ=1
PAL_TBL                  len= 16 ROM 00:9B83 occ=1
TITLE_ROUTINE_PTR_TABLE  len= 62 ROM 00:802A occ=1
PASS_WORD_VRAM           len= 21 ROM 00:937F occ=1
BANK_TABLE               len= 16 ROM 07:CA30 occ=1
ITEM_DATA                len=118 ROM 00:9029 occ=1
MAIN_ROUTINE_PTR_TABLE   len= 20 ROM 07:C141 occ=1
PROJECTILE_OFFSET_X      len= 10 ROM 01:9CBD occ=2
METROID_LATCH_OFFSET_Y   len= 12 ROM 03:99D8 occ=1
INSTR_LISTS              len= 45 ROM 03:9D99 occ=1
SPAWN_POS_TBL            len= 12 ROM 03:A1DB occ=1
```

（12 张表；`BANK_TABLE` 两处常量字节相同，都命中 `07:CA30`。）

`PROJECTILE_OFFSET_X` 命中两处：`01:9CBD` 与 `04:9BD1`——这是区域 bank 的共享副本（bank1 Brinstar / bank4 Kraid hideout），字节逐一对同（`occ=2`）。迁移时以区域 bank 为准，不做全局唯一假设。

### 2.3 指针表语义（A）

- `TITLE_ROUTINE_PTR_TABLE`：ROM `00:802A` 起 31×u16，值本身就是**绝对地址**（如 `8071 80D0 … C4AA`）；消费方 `title/bridge.rs:94` 只把它写进 `$0C/$0D` 交给 `JumpEngine`。
- `MAIN_ROUTINE_PTR_TABLE`：ROM `07:C141` 起 10×u16（`C801/C81D/C8D1/C92B/C9A6/C9B1/C9C4/C155/C9D7/C494`），asm `MainRoutinePtrTable ; 07:C141` 逐项 `PtrTableEntry MainRoutinePtrTable, <label>`（`prg7_engine.asm:347-357`），同样是绝对地址。
- `ITEM_DATA`：59×u16（`00:9029`），消费方 `bridge.rs:340` 用 `< ITEM_DATA.len()` 守卫、`x==59` 时走 `ITEM_DATA_OOB`（asm 的越界读行为），迁移后守卫与越界分支必须保留（C1）。

## 3. 超读：`BANK_TABLE` 的 16 B 里只有 5 B 是表

asm（`prg7_engine.asm:2274-2279`）：

```
BankTable: ; 07:CA30
    .byte $01+1     ; Brinstar
    .byte $02+1     ; Norfair
    .byte $04+1     ; Kraid hideout
    .byte $03+1     ; Tourian
    .byte $05+1     ; Ridley hideout
;---[ Saved game routines (not used) ]---
```

ROM 该址 16 B：`02 03 05 04 06 | 48 48 20 96 CA AD 84 68 10 0D 29`——前 5 B 是表，后 11 B 是紧随的代码（`PHA PHA JSR …`）。
两份 Rust 常量（`title/bridge.rs:728`、`frame/tick.rs:798`）都抄了 16 B。消费方只用 `[0..5]`（`MainRoutine`/`BankSwitch` 的 4 LSB），行为未受影响；但**按表烘焙就会把代码当数据**，与 asm 真长不符。迁移时应裁到 5 B（或 8 B 补 `$00` 对齐并注明），并把「有用 5 项」写进 PackKey 注释。

## 4. 派生常量（6 条，不进 ROM 导出）

| 常量 | 结论 | 依据 |
|---|---|---|
| `PW_NOERR_MSG [0xFF;15]` | 派生：无错时把 15 个密码格填 `$FF`（空白），不是 ROM 表 | 全仓库无 ROM 序列；asm 侧「无错」路径只做填充 |
| `JB_CHARS` | 派生：`"JUSTIN BAILEY"` 的密码字码（`0x13,0x1E,0x1C,0x1D,0x12,0x17`…）后接 12 个 `0x3F`（空格） | 由密码字符表映射逐字推出，ROM 无此连续序列 |
| `RESPAWN_ANIM_INDEX [6,7,8,0,1,2,3,4,9,5]` | 派生：砖块重生动画顺序 | ROM 无此序列；由 `set_tile_anim` 的 slot 语义推出 |
| `LENGTH_TABLE [32]` | 硬件规格：NES APU 长度计数器查表（`10,254,20,2,…`） | 是 2A03 的定义表，不是本 ROM 的数据 |
| `TRI_SEQUENCE [32]` | 硬件规格：三角波 32 步序（0..15,0..15） | 公式 `i<16 ? 15-i : i-16` |
| `NOISE_PERIODS [16]` | 硬件规格：NTSC 噪声周期表（`4,8,16,32,64,96,…`） | 是 2A03 NTSC 周期表 |

这 6 条**保留在 Rust 里**，理由：它们不是 ROM 内容，而是硬件定义/由文本或算法派生；搬进 `assets/metroid.pack` 反而会让「pack 与 ROM 一一对应」的校验失去意义。若以后要统一，应在 `game-platform` 侧写成生成函数而不是表。

## 5. 非 ROM 数据（3 条，保留）

- `samus/state.rs:334 TYPED_ADDRS [u16;23]`：Rust 侧 typed 访问器的地址清单（`SAMUS_STATUS` 等常量组合），不是从 ROM 抄的字节。
- `frame/tests.rs:1056 T [u8;40]`、`enemy/state.rs:888 TBL [u8;18]`：都在 `#[cfg(test)] mod tests` 里，是夹具。

## 6. 迁移动作（每条：PackKey → extract 函数 → 运行时落点）

| # | 常量 | 新 PackKey | extract 函数/取址 | 运行时落点 | 备注 |
|---|---|---|---|---|---|
| 1 | `DATA_DISPLAY_TBL` | `HudTemplate` | `extract_hud_tables()`：`bank_slice(prg,7,0xE1B9,40)` | `ui/hud.rs` 改 `pack::get(PackKey::HudTemplate)`（`Option` ⇒ 缺失 fail-loud） | 40 B 整表 |
| 2 | `NAR_PASSWORD_TBL` | `NarPassword` | `bank_slice(prg,0,0x8E07,16)` | `ui/password.rs` 三处引用（比较/写入） | 与 `PwErr`（`8759`）同批 |
| 3 | `PAL_TBL` | `TitlePal` | `bank_slice(prg,0,0x9B83,16)` | `title/mod.rs:1120` | — |
| 5 | `TITLE_ROUTINE_PTR_TABLE` | `TitleRoutinePtrs` | `bank_slice(prg,0,0x802A,62)` | `title/bridge.rs:94`（`u16` LE 解码） | 值=绝对地址 |
| 6 | `PASS_WORD_VRAM` | `PwVramStruct` | `bank_slice(prg,0,0x937F,21)` | `title/bridge.rs` 的 VRAM 写入 | 与 `TITLE_VRAM` 同函数族 |
| 8/14 | `BANK_TABLE`×2 | `AreaBankTable` | `bank_slice(prg,7,0xCA30,5)` | `title/bridge.rs`、`frame/tick.rs` | **裁到 5 B**（§3） |
| 9 | `ITEM_DATA` | `ItemData` | `bank_slice(prg,0,0x9029,118)` | `title/bridge.rs:340/760` | 守卫+`ITEM_DATA_OOB` 保留 |
| 13 | `MAIN_ROUTINE_PTR_TABLE` | `MainRoutinePtrs` | `bank_slice(prg,7,0xC141,20)` | `frame/tick.rs:357` | 热路径，见 §7 批 3 |
| 15 | `PROJECTILE_OFFSET_X` | `KraidProjOffsetX` | `area_bank_slice(prg,1,0x9CBD,10)`（Kraid 用 bank4） | `enemy/kraid.rs:68` | 两副本同字节（§2.2） |
| 16 | `METROID_LATCH_OFFSET_Y` | `MetroidLatchOffsetY` | `bank_slice(prg,3,0x99D8,12)` | `enemy/metroid.rs:50` | — |
| 17 | `INSTR_LISTS` | `CannonInstrLists` | `bank_slice(prg,3,0x9D99,45)` | `enemy/cannon.rs:22` | — |
| 19 | `SPAWN_POS_TBL` | `RinkaSpawnPos` | `bank_slice(prg,3,0xA1DB,12)` | `enemy/rinka.rs:309` | — |

### 6.1 消费侧统一形态

现有消费范例（`crates/game-data/src/assets.rs:569`）：

```rust
let t = pack::get(PackKey::RomTable968B);
```

迁移后的 12 处都改这个形态；`pack::get` 返回 `Option<&'static [u8]>`，缺段要 `expect`/`panic`（fail-loud），**不许**回退到硬编码常量（否则等于双源）。

### 6.2 顺带移除的隐患

- `BANK_TABLE` 超读（§3）：裁 5 B 后，`title/bridge.rs` / `frame/tick.rs` 的 `[5..]` 无消费者 ⇒ 删除即无行为差。
- `PROJECTILE_OFFSET_X` 的区域副本：迁移时明确「取哪一份」，避免以后有人用 bank1 的副本改 bank4 的数据。

## 7. 分批与验收判据

| 批 | 内容 | 风险 | 判据 |
|---|---|---|---|
| **批 1（低）** | #1 `DATA_DISPLAY_TBL`、#2 `NAR_PASSWORD_TBL`、#3 `PAL_TBL`、#5 `TITLE_ROUTINE_PTR_TABLE`、#6 `PASS_WORD_VRAM`、#8/#14 `BANK_TABLE` | 标题/HUD 静态表，不进逐帧热路径（`BANK_TABLE` 只在换区时读） | 删除 7 条常量（含两份 `BANK_TABLE`）⇒ `cargo test -p game-core` 全绿；`run-all` 仍 171/172；`title` 相关 L4 case（`case-title-*`）逐字节不变 |
| **批 2（中）** | #9 `ITEM_DATA`、#15 `PROJECTILE_OFFSET_X`、#16 `METROID_LATCH_OFFSET_Y`、#17 `INSTR_LISTS`、#19 `SPAWN_POS_TBL` | 敌人/密码面，区域 bank 数据 | 删除 5 条 ⇒ Kraid/Rinka/Cannon/Metroid 对应 L4 case 不变；`run-all` 不回退；`pack-tool verify` 通过（新段有 meta 记录） |
| **批 3（高）** | #13 `MAIN_ROUTINE_PTR_TABLE` | **逐帧热路径**：`frame/tick.rs:357` 每帧用 `MainRoutine` 取目标 | 删除 1 条 ⇒ `cargo test` 全绿、`run-all` 171/172、`case-extended-*`/`case-intro` 不变；`pack::get` 在 `GameEngine` 初始化时取一次并缓存引用（不得每帧解析） |

**热路径标注**（必须在 PR 里写明）：`MAIN_ROUTINE_PTR_TABLE`（每帧）、`BANK_TABLE`（每次换区）、`ITEM_DATA`（`UniqueItemSearch` 每次进密码/暂停面 60 次循环）、`SPAWN_POS_TBL`/`INSTR_LISTS`/`PROJECTILE_OFFSET_X`（敌生成/开火时）。`TITLE_ROUTINE_PTR_TABLE`/`PAL_TBL`/`PASS_WORD_VRAM`/`DATA_DISPLAY_TBL`/`NAR_PASSWORD_TBL` 只出现在标题/HUD 面，风险最低。

## 8. 复现

见 §1.1、§2.1 的两条命令；另：

```bash
# 超读证据：BankTable 真长 5 B
sed -n '2274,2279p' refs/m1disasm/SRC/prg7_engine.asm
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=7*0x4000+(0xCA30-0xC000);print(' '.join(f'{b:02X}' for b in prg[o:o+16]))"
# → 02 03 05 04 06 48 48 20 96 CA AD 84 68 10 0D 29   （前 5 B 是表）
```

## 9. 未覆盖与 C 级假设

- **C1**：`ITEM_DATA` 第 60 项（`x==59`）的 `ITEM_DATA_OOB` 值是从 asm 越界读推导的（`bridge.rs:343`），迁移后必须保留该常量与分支——若导出器只导 59 项，越界分支的值仍需本地定义（这是「派生」，不是 ROM 数据）。
- **C2**：`PROJECTILE_OFFSET_X` 在 bank1/bank4 各一份，本文只核了字节相同；哪一份是「主副本」属命名问题，未取证。
- **C3**：`DATA_DISPLAY_TBL` 40 B 对应 asm 的 HUD 模板表（`DisplayHUD` 未端口），段边界按 40 B 匹配唯一，但该表无独立 asm 标签，标签名为本文拟。
- 未做：`crates/game-data/src/extract.rs` 的新 extract 函数**没有实现**（本卡只审计）；`pack-tool extract` 的段数变化需在迁移卡里同步 `SegMeta`。

## 10. 证据指针

- ROM 匹配：§2.1 脚本 + §2.2 输出（12 张表，`occ` 见输出）。
- asm 锚：`prg7_engine.asm:2274-2279`（BankTable 5 B）、`:347-357`（MainRoutinePtrTable 10 项）、`constants_ram.asm`（相关 RAM 名）。
- 消费点：`ui/hud.rs:36`、`ui/password.rs:207/265/283`、`title/mod.rs:1120`、`title/bridge.rs:94/340/595/760`、`frame/tick.rs:357/397`、`enemy/kraid.rs:68`、`enemy/metroid.rs:50`、`enemy/cannon.rs:22`、`enemy/rinka.rs:309`。
- 现有导出范例：`crates/game-data/src/extract.rs:1081-1145`（`TITLE_TABLES`/`TITLE_VRAM`）、`:985-1000`（`extract_rom_tables`）、`:1100-1104`（`SAMUS_TABLES`）；消费范例 `crates/game-data/src/assets.rs:569`。
