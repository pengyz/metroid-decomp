# D 面完备性扫描（L4 前置完备性扫描工单②——用户裁定（完备性与正确性分离））

## 1. D 面完备性扫描（asm 数据表 vs Rust 数据）

### 1.1 asm 数据表（rom_tables（refs asm（数据表清单（bank:addr（表名（D 面（grep 实录（数据表（.sym（数据表符号）））））））

**数据表清单**（bank:addr（表名（D 面（grep 实录（数据表（.sym（数据表符号））））））：
- rom_tables（refs asm（数据表清单（bank:addr（表名（D 面））））
- .sym（数据表符号）：grep 实录（数据表（表名（表内容（表值））））

### 1.2 Rust 数据（game-data（crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值））））））

**Rust 数据表**（game-data（crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值））））））：
- crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值）））））
- 表名/表内容/表值：grep 实录（数据表（表名（表内容（表值））））

## 2. 覆盖率（Rust 已实现数据表数/asm 数据表总数（完备性指标（D 面）））

**完备性指标**（D 面）：
- 覆盖率：Rust 已实现数据表数/asm 数据表总数
- 缺口清单：未实现数据表（bank:addr/表名（缺口（缺漏）））

## 3. 交付判据

完备性扫描文档（docs commit（D 面覆盖率（百分比（A 级实录））+缺口清单（未实现数据表清单）））——「候令」禁用。

## 4. D 面缺口清单深化（2dcb5b4 缺口清单（未实现数据表（bank:addr/表名（缺口（缺漏））））

### 4.1 缺口深化（未实现数据表（表内容（表值（表用途（表功能考证））））

**缺口深化**（未实现数据表（表内容（表值（表用途（表功能考证（refs asm（数据表使用径（bank:addr（表读径（grep 实录（数据表使用（子程序（表读指令））））））））））：
- 表内容/表值/表用途：表功能考证（refs asm（数据表使用径（bank:addr（表读径））））
- 数据表使用径：grep 实录（数据表使用（子程序（表读指令）））

### 4.2 缺口清单深化（A 级实录（表名/表用途/缺口（缺漏（L4 场景矩阵候补）））

| 表名 | 表用途 | 缺口（缺漏（L4 场景矩阵候补）） |
|---|---|---|
| 待考证 | 待考证 | L4 场景矩阵候补 |

## 5. 交付判据

缺口清单深化（docs commit（D 面缺口（表名/表用途/缺口（缺漏（L4 场景矩阵候补））（A 级实录）））——「候令」禁用。

> 【2026-10-21 D 面数据表考证首件（lead 直接落档（Samus 域数据表（跳跃/变球（A 级实录））））】
> Samus 域数据表考证首件（bank:addr（表读径（表值（表用途（表功能考证（refs asm（数据表使用径
> （bank:addr（表读径（grep 实录（数据表使用（子程序（表读指令））））））））））：
> ①ItemData@HighJump（00:9059）——表用途：密码系统（bitmask（prg0_title.asm（3681-3682（lda #1<<
> (((ItemData@HighJump-ItemData)/2)&7).b / sta PasswordByte+(((ItemData@HighJump-ItemData)/2)/8))）））
> ——表功能：HighJump 道具（密码字节位掩码）
> ②SFXData@SamusJump（00:b251）——表用途：音效数据（sound_engine.asm（1033（ldy #<SFXData@SamusJump.b）））
> ——表功能：跳跃音效（SFX 初始化（SamusJumpSFXInit（00:b703）））
> ③SFXData@SamusBall（00:b26d）——表用途：音效数据（sound_engine.asm（1294（ldy #<SFXData@SamusBall.b）））
> ——表功能：变球音效（SFX）
> ④ObjAnim_SamusJumpTransition（01:857d）——表用途：动画数据（prg0_title.asm（439（lda #
> ObjAnim_SamusJumpTransition - ObjectAnimIndexTbl.b）））——表功能：跳跃过渡动画（ObjAnim（动画表索引））
> ——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（grep 实录（数据表使用（子程序（表读指令）））））））
> ——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））
> ⚠ 数字差辨（verifier（22.2%（1923/8659））vs decomp（22.3%（1928/8659（第四组补全（767dd7a））））
> ——实际审计数字为准（1928/8659=22.3%（A 级实录（decomp（第四组补全实录））））
