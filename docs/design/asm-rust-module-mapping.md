# asm→Rust 模块映射章程（词汇裁决·三层骨架·归宿表·重排闸门）

> 状态：v1（2026-10-24）。上游：phase0-architecture-adversarial.md（划分原则）+
> data-driven-rom-alignment.md（归宿表）。对抗复核 verdict=needs_revision
> （F1-F3/M1-M4）——本章程为其处置产物：F3 词汇裁决先行（§1）、F1 三层骨架（§2）、
> M1/M2/M4 缺口与归宿登记（§3-4）。D-a 删除以 F2 正名理由执行（见 §6）。

## 1. 词汇裁决（F3 首要件——三形归一）

现状三形自相矛盾（phase0:33=`kernel/`；data-driven-gamecore:11/14=`kernel/`+
`engine/{frame,nmi,boot}` 两处自称"帧管线核心"；data-driven-architecture:47-49=只有
engine/；簇11=kernel::frame、簇18/19=engine::*）——裁决如下，**本文为正典**：

| 词 | 裁决语义 | 依据 |
|---|---|---|
| **`frame/`** | **周期级忠实引擎（帧管线核的正典名）**：tick 状态机/NMI 体/boot 链/主循环迭代/标题桥/声钩——**L3/L4 验证路径唯一实现体**（七金面零差/L3 双腿） | 实证：frame/ 五构件落地（复核④）；且 `frame→engine` 改名被占名碰撞否决（F1：engine/frame.rs 已存） |
| **`engine/`** | **1:1 转写底座**（E8 时代 521 机械名去地址名批改产物）：L2 安全网（l2ports/verify_shim 直调）+域件共享叶（enemy/samus 引用）——随域改写逐簇收编，Stage A 不突删 | F1：mod.rs 616KB+frame.rs 288KB 活引用实证 |
| **`kernel/`** | **退役**（被取代的早期折叠件）——历史文档中 `kernel/` 词汇一律以本文校正（原主张原地 dated-correction 标注，不删历史） | F2+M3：锚归宿已核全（见 §6） |

**同名互指消歧**（登记）：`engine/frame.rs`（转写底座·帧件）vs `frame/engine.rs`
（FrameEngine 状态机）——两件语义不同名相撞；Stage B 重排时更名（候选：
engine/frame.rs→engine/frame_slices.rs）；此前引用处以全路径消歧。

## 2. 三层骨架（F1——架构现状的正典分层）

```
game-core
├── ①转写底座   engine/（1:1 切片直译·L2 安全网·域共享叶）
├──②忠实引擎   frame/（周期级集成体·L3/L4 对拍路径）← 帧管线核正典
├──③域端口层   title/ sound/ enemy/ samus/ world/ weapon/ pickup/ ui/ ending/ input/
│              （域方法+纯函数；frame/ 会师点与 L2 面双消费）
├── 契约层      memmap/ + ram_map.rs（地址纪律——两层架构唯一地址层）
├── 验证基建    verify_shim/ + l2ports.rs（**非游戏代码**——L2In/L2Out 写集投影与案件）
├── 新构件      render/（软件 PPU 光栅器——非 asm 映射物，按 phase0 E9 新建；
│              8-sprite quirk/优先级=D4 保真）
└── 烘焙面      game-data crate（ROM 直读解析——禁手抄）
```

## 3. 映射轴与归宿表

**三轴**（沿 phase0:28-29 逐字）：①帧管线权威序（E3 UpdateWorld 19 子系统/E4 NMI 相）
②RAM 结构分区（constants_ram .struct/.enum）③调用图/数据流——**不以 bank、不以文件定边界**。

| asm 归宿 | Rust 归宿 | 备注 |
|---|---|---|
| prg7_engine.asm 帧管线核（NMI/MainLoop/UpdateTimer/Startup） | `frame/` | 集成体；engine/ 另存 L2 切片（双实现登记见 §4-R1） |
| prg7_engine.asm 域例程（Samus/Enemy/Door/Elevator/…） | `samus/` `enemy/` `world/` 等域 | 驱动在 prg7、AI 在 enemies/——边界按调用图 |
| prg0_title.asm | `title/` | **M2 漂移登记**：phase0 曾规划归 ui/；现实 title/+ui/password.rs 分居——以现实为准，Stage B 再并 |
| enemies/*.asm（16 AI） | `enemy/` | 一一对应 |
| sound_engine.asm | `sound/` | |
| 区域数据目录/common_sprite_data/songs/CHR | game-data（烘焙面） | ROM 直读 |
| constants_ram.asm | `memmap/`+`ram_map.rs` | 零页二分铁律（暂存→局部/持久→字段） |
| PPU 硬件行为（无 asm 对应物） | `render/` | 新构件声明 |

## 4. 缺口登记（M1+清理待办——每条带义务）

| # | 缺口/待办 | 义务 |
|---|---|---|
| G1 | **帧预算记账未建**（M1——phase0-zero-page-frame-budget.md 设计在档、代码零实现） | 掉帧语义进 frame/ 前，重负载 L4 场景分叉风险显式登记 |
| G2 | **M6 超帧声路缺席**（nmi_body:40-42 早退吞 SoundEngine/UpdateAge；E4 表=门外无条件） | 声域波次内修（已入声域缺口表） |
| G3 | **port-map registry 未建**（phase0 交付物③：data/naming/port-map.toml+tools/port_coverage.py） | S 面分母×registry 双射门禁 |
| G4 | **R1 双帧引擎**（engine/frame.rs L2 切片 vs frame/ 集成体——本轮迭代序门修复只落 frame/，两实现已分叉） | 归并或显式声明"L2 切片独立保真、分叉面登记"——须独立复核轮+L2 案全绿 |
| G5 | engine/nmi.rs s1_s2_family v1-v5 五代并存 | 裁剪须先证覆盖含属（各代独有断言清单）——测试面零回退义务 |
| G6 | 散件归位（integrators/tile_blast/rng/gap） | move-only commit+测试随迁零 delta |
| G7 | 同名互指更名（§1 消歧登记） | Stage B |

## 5. Stage A/B 重排闸门

**Stage A（行为一致期）**：模块结构只增不重排——改名/移动仅限消歧与死件；
行为变更走 TDD+对拍（L2/L3/L4 基线不许动）。**Stage B（现代化期）**才允许
结构性重组（域合并/接口 trait 化/帧预算记账入核）——以本文 §2 骨架为重组基。

## 6. 清理动作账（本轮执行——复核处置实录）

| 动作 | 复核处置 | 测试面影响 |
|---|---|---|
| 删 kernel/+game_logic.rs | **F2 正名**：被取代的早期折叠件（非"自造件"——两件均有 A 级锚声称）；逐锚归宿已核：F10C/F1FA/F284→enemy/ai.rs（同三锚更强家）+engine/mod.rs 员体+verify_shim::f1fa_with_carry；C01A→frame/boot.rs:47+engine/boot.rs:40（双重更强家） | -13 例（kernel 8+game_logic 5——随删清单：exhaustive_startup_writes/boundary_indices/…）；清 RAM 语义由 frame/boot.rs clear_ram_lo_zero_fills_pages 已覆盖（M3） |
| NeedsBgKernel→NeedsBgCollision | M4：一词二义消歧（瓦片环碰撞核 vs 帧内核）；且该词系我方造词（asm=CheckMoveVertical 无 kernel 词） | 0（测试随型改名） |
| 文档词汇校正 | F3：phase0/data-driven 系 kernel 词汇 dated-correction 注 | 0 |
