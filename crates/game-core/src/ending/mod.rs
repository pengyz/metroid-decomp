//! 结局链（B9）实现面——规格 `docs/spec/ending-routine.md`（#78，A 级取证）。
//!
//! 规格 §6 已核过：`EndGame` 七段骨架、`ChooseEnding`、`UpdateAge`、精灵装载都已在
//! `title/` 与 `frame/nmi.rs` 端口；真正没进 Rust 的是**结局背景/职员表的 PPU 写面**
//! （`VRAMStruct_EndBackground ; 00:A052`、`LoadCredits ; 00:9C45` 的 44 项索引）与
//! **结局调色板的选择规则**。本模块只补这三块，并按卡面边界不碰 `title/`。
//!
//! # 决定点清单（`~`=本卡补，`=`=已端口不在本卡，`x`=不做）
//!
//! | 决定点 | ROM | 状态 |
//! |---|---|---|
//! | `$8F` 电梯触发（`AtEnding++`、`TitleRoutine=0x1D`） | `07:D8BF` | = `world/elevator.rs:274-284` |
//! | `EndGame` 每帧骨架 + 七段脚本 | `00:9AA7` | = `title/mod.rs:689`、`:1033-1260` |
//! | 段 0 结局背景 VRAM（`A052`，17 记录） | `00:9AD5` | ~ [`vram::background_records`] |
//! | 段 5 职员表索引（`CreditsPtrTable` 44 项） | `00:9C45` | ~ [`credits::record_index`] |
//! | 结局调色板循环（每 16 帧，`$09`→`$01`） | `00:9AA7` 头部 | ~ [`palette::next_pending`] |
//! | 淡出/职员表调色板档位（`$0C`/`$0D`） | `00:9BCD`、`00:9BFC` | ~ [`palette::PaletteSlot`] |
//! | `ChooseEnding` + `AgeTable` 阈值 | `07:CAF5`、`07:CB18` | = `title/mod.rs:1941`（含原版 `+3` 不查的 bug 保真，[`choice`] 只做 typed 包装） |
//! | `UpdateAge`（`$D0` 进位链） | `07:C97E` | = `frame/nmi.rs:385` |
//! | 结局精灵六帧 / 挥手四表（OAM 写面） | `00:9C9A`、`00:9C7F` | = `title/ending_sprites.rs` |
//! | 段 5 滚动/翻页/收尾 | `00:9BFC` | x 见 [`credits`] 模块文档：现有滚动逻辑在 `title/mod.rs:1211-1240`，改它属 `title/` 写面 |
//! | `LoadCredits` 在 NMI 里补露出的 4 行 | `00:9C45` | x 接线属 `frame/nmi.rs`（仍是 `unported!("bank00:9C45")`），另卡 |
//! | 规格 §6.1 的私有槽号缺陷（`0x0F..0x18` 当地址、`$0014/$0015` 被覆盖） | `00:9AD5` 等 | x 写面在 `title/`；本卡清单只登记，不修 |
//!
//! 规格 §8 的 C 级假设（`EndFadeOut` 的 `IsCredits←1`、段 1 首帧旧调色板、`$6A00`）
//! 一律不实现：本模块每个常量都能指到上表的 asm 地址。

pub mod choice;
pub mod credits;
pub mod palette;
pub mod vram;
