# Phase 0 架构设计方案（对抗性设计·独立架构师产出）

> 角色：fresh-context 独立架构设计师。产出=可执行 Phase 0 架构 + 对抗性自检。
> 证据基础：refs/m1disasm 实读（SRC 结构 / prg7_engine.asm 管线 / constants_ram.asm 结构定义）
> + crates/game-core 实读（两层架构 spec 与现状代码形态）。
> 本文是设计提案，落库需经独立复审（AGENTS §5）；其中 spec/registry 产物需按 §9 提交规范带 Evidence 字段。

## 0. 实证基础（本设计引用的 A/B 级证据）

| # | 事实 | 证据 |
|---|---|---|
| E1 | m1disasm 物理组织 = 8×16KB bank（prg0_title / prg1_brinstar / prg2_norfair / prg3_tourian / prg4_kraid / prg5_ridley / prg6_graphics / prg7_engine）+ 区域数据目录（brinstar/norfair/tourian/kraid/ridley 各含 rooms/structures/structure_ptrs/room_ptrs/items/palettes/metatiles/enemy_sprite_data）+ enemies/（16 敌 AI）+ songs/ + sound_engine.asm + areas_common.asm + common_sprite_data.asm | refs/m1disasm/SRC 目录 + linkfile |
| E2 | prg7_engine.asm（12930 行）含 86 个语义 banner 节——**语义模块边界社区已标注** | `grep ';.*\[ .* \]' prg7_engine.asm` |
| E3 | 帧管线权威顺序：`UpdateWorld`（bank7:$CB29）= UpdateAllEnemies → UpdateSamusProjectiles → UpdateSamus → AreaRoutine → UpdateElevator → UpdateAllStatues → UpdateAllEnemyExplosions → UpdateAllMellows → UpdateAllEnProjectiles → UpdateAllSkreeProjectiles → SamusEnterDoor → UpdateAllDoors → UpdateAllTileBlasts → CollisionDetection → DisplayBar → UpdateAllPipeBugHoles → CheckMissileToggle → UpdateAllPowerUps → UpdateTourianItems → 清尾部 SpriteRAM | prg7_engine.asm:2492-2530 |
| E4 | NMI（bank7:$C0D9）顺序：OAMDMA($0200) →（NMIStatus==0 才做：NMIScreenWrite→CheckPaletteWrite→CheckVRAMStructBufferWrite→WritePPUCtrl→WriteScroll→ReadJoyPads）→ SoundEngine → UpdateAge → NMIStatus=1。**NMIStatus=帧完成旗，超时帧跳过 vblank 后半（掉帧语义）** | prg7_engine.asm:225-294 |
| E5 | RAM 已有社区类型化结构：OAMSprite/Object/En/EnExtra/EnExplosion/TileBlast/PowerUp/Zebetite/Cannon/PipeBugHole/RinkaSpawner/Mellow/SkreeProjectile/IntroSpr/AreaPointersStruct 等 .struct；.enum 分区：$0200 SpriteRAM(64 sprites) / $0300 Objects(16×16B：Samus/Elevator/PowerUpDraw/Statue/Doors×4) / $0310 Samus 亚像素物理 / $0400 Ens(6)+EnProjectiles(6)+EnExplosions(4)+MotherBrain / $0500 TileBlasts(13) / $0600 声音引擎 / $0700 杂项(PipeBugHole/PowerUp/Zebetite…) / $6000 RoomRAM A/B / $7000 WorldMapRAM | constants_ram.asm:1-243, 804-1303 |
| E6 | **零页 $00-$11 是按例程上下文反复别名的暂存区**（同一 $04 = Temp04_YSlotRadiusY / Temp04_SpeedY / Temp04_RoomRAMPtr…）；$12 起才是持久命名状态（Joy*/NMIStatus/GameMode/MainRoutine/CurrentBank/Timer*/RandomNumber*/RoomPtr/StructPtr/Scroll*/MirrorCntrl…） | constants_ram.asm:244-460 |
| E7 | 分派机制 = JumpEngine 间接跳 + 指针表（MainRoutinePtrTable=AreaInit/MoreInit/SamusInit/GameEngine/GameOver/Pause/Password/…；敌/门处理、电梯、Samus handler 均如此） | prg7_engine.asm:687-713, 345-355 |
| E8 | 现状代码形态 = bank:addr 切片转写（l2ports.rs `sub_c16d`、engine/mod.rs `s1_*` 共 521 个机械名 fn；l4_ready_impl.rs `SamusState` 平铺 60+ 字段混装 samus/enemy/render 状态）——函数级 6502 翻译，语义域结构未成形 | crates/game-core/src/{l2ports,engine/mod,samus/l4_ready_impl}.rs |
| E9 | 两层架构 spec 已立（产品层禁裸地址 / memmap 地址只此一层 / verify_shim 写集契约；行为钉死=结构可换） | docs/spec/two-layer-architecture.md |

---

## Phase 0 架构设计方案

### 模块划分方案

**划分原则**：逻辑功能域 = Rust 模块；**不镜像物理 bank、不镜像 asm 文件**。
bank:addr 只保留在 memmap/registry（供 L2 定位与 S 面缺口核查）。模块边界以 UpdateWorld/NMI 管线（E3/E4）+ RAM 结构分区（E5）+ 调用图/数据流定，不以文件定（door 逻辑散在 areas_common+prg7；敌 AI 在 enemies/ 但驱动在 prg7）。

```
crates/game-core/src/
├── kernel/    帧调度（tick 两相）·NMIStatus 帧同步·Timer*/SetTimer·JumpEngine→enum 分派·bank 窗口抽象·RNG
│              > 【2026-10-24 更正】词汇归一至 docs/design/asm-rust-module-mapping.md §1：
│              > 帧管线核正典名=**frame/**（本行 kernel/=同义历史词）；kernel/ 脚手架已退役。
├── input/     手柄读取/连发（Joy1/2 Status/Change/Retrig/RetrigDelay）
├── render/    SpriteAllocator（SpritePagePos 游标契约）·OAM 影子(64 sprites)·VRAMStruct 写队列(RLE)·
│              palette 队列·scroll/镜像·**软件 PPU 光栅器**（平台无关 VideoSink）
├── world/     房间加载（RoomRAM A/B、结构→元块→属性表）·门（进 door/滚动 door/UpdateAllDoors）·
│              电梯·石像·卷轴/nametable 切换·世界地图·区域策略（AreaRoutine: brinstar/…/tourian）
├── samus/     Samus 状态机（standing/jump/roll/ball/salto…handler 表）·物理积分（subpixel/accel/max speed）·
│              受伤/血量/能量罐·熔岩/地形交互
├── weapon/    Samus 弹幕（missile/bomb/wave 轨迹指令串）·tile 爆破（TileBlasts 13 槽）
├── enemy/     敌槽（Ens 6）·AI（crawler/dragon/kraid/metroid/multiviola/pipe_bug/polyp/ridley/rinka/rio/
│              ripper/sidehopper/skree/squeept/swooper/waver → enemies/*.asm 一一对应）·
│              敌弹（6 槽）·爆炸（4 槽）·Mellow·掉落（Item drop table）
├── pickup/    PowerUp（2 槽）·Tourian items·特殊道具（Setup special items / PowerUp Handler）
├── ui/        状态栏（DisplayBar/hex→dec/能量罐）·标题/开场（星星/十字准星）·结局·密码·暂停·Game Over
├── sound/     音乐引擎 + SFX（$0600 状态区）→ APU 寄存器流抽象（独立验证面）
└── (game-data crate) ROM 数据表：room_ptrs/structure_ptrs/metatiles/palettes/enemy_sprite_data/
    anim frame tables/EnemyMovement 指令串/songs —— 从 ROM 字节解析，不手抄
```

模块间接口（共享状态对象 + 域方法；避免过度 trait 化）：

```rust
// kernel：帧入口（确定性时钟）
pub struct GameState { /* 各域状态聚合，见 §asm→Rust */ }
pub struct FrameInput { pub joy1: u8, pub joy2: u8 }
pub struct FrameOutput { pub video: VideoFrame, pub apu: ApuFrame } // 平台无关
pub fn tick(state: &mut GameState, input: &FrameInput, out: &mut FrameOutput);

// render：精灵分配游标契约（E3 中所有 update 共享 SpritePagePos）
pub struct SpriteAllocator<'a> { buf: &'a mut [Sprite; 64], cursor: &'a mut u8 }
impl SpriteAllocator<'_> {
    /// 对应 SpritePagePos 推进语义；耗尽行为按 asm 保真（wrap/丢弃）
    pub fn alloc(&mut self, n: usize) -> Option<&mut [Sprite]>;
}

// enemy：16 敌 AI 一一实现（enemies/*.asm ↔ enemy::ai::<name>）
pub trait EnemyAi {
    fn update(&self, e: &mut Enemy, ctx: &mut EnemyCtx<'_>);
    // EnemyCtx 携带：sprites（SpriteAllocator）, room（碰撞查询）, rng, sfx 请求, samus 位置
}

// world：门/房间
impl World {
    pub fn samus_enter_door(&mut self, samus: &mut Object, ctx: &mut FrameCtx);
    pub fn update_doors(&mut self, sprites: &mut SpriteAllocator, ctx: &mut FrameCtx);
    pub fn room_collision(&self, x: u8, y: u8) -> TileKind; // 吸收 GetPPUAndRoomRAMAddresses 族
}

// sound：APU 流
pub struct ApuFrame { pub writes: Vec<(ApuReg, u8)> } // 帧内寄存器写序列，用于对拍
```

集成契约：kernel 按 **E3 顺序**调用各域 update、按 **E4 顺序**执行 vblank 相——顺序本身是规格（写进 spec 并有 L3 场景钉住）。

### asm→Rust 对应关系

| 6502 构造 | Rust 等价 | 设计决策（含对抗性修正） |
|---|---|---|
| **零页暂存 $00-$11**（TempNN_* 上下文别名区） | **函数局部变量 / 函数参数**（如指针传参 $00-$01 → `&mut T`） | ⚠ 不许做成 struct 字段。E6 证明同一地址按例程换语义——语义是「调用域暂存」。做成字段=把偶然存储复用固化为隐式跨调用通道（l4_ready_impl 平铺 SamusState 正是此反例） |
| **零页持久 $12+ / $0106+ / 分区 RAM** | struct 字段，**语义分组**（非地址序）：`FrameSync{nmi_status,ppu_data_pending,palette_pending}`、`Input{…}`、`Scheduler{game_mode,main_routine,title_routine,next_routine,timers}`、`Scroll{x,y,mirror,nametable}`、`Banking{current,pending}`、`Rng{r1,r2}` + E5 的 Object/En/TileBlast… 各自成 struct | 地址只在 memmap 层 encode/decode（E9 两层架构不变） |
| **子程序** | impl 方法（语义命名）：UpdateSamus→`samus::Samus::update()`；int_y($E3A5)→`samus::physics::integrate_y()`；CollisionDetection→`collision::resolve_all()` | 机械名 bank:addr 只活在 registry 注释；命名走 data/naming 台账（A/B 级证据，D5） |
| **数据表** | game-data 从 ROM 解析 → typed const/结构（RoomPtrs/MetatileSet/AnimFrame/MovementInstr/Song） | 禁手抄数值；D 面对拍=解析产物 vs ROM 偏移（沿用 d-face 工作流） |
| **bank 切换** | 逻辑分组（模块=域）；bank 事实留在 `memmap::banking::WindowBank`（L2In.window_bank 已参数化）；区域差异→`world::area::*` 策略表 | 跨 bank 调用=普通 Rust 调用；registry 记 bank:addr→crate::module::fn 双射 |
| **NMI 同步** | `tick()` 两相：vblank 相（E4 顺序）+ 主相（GoMainRoutine→UpdateWorld）；NMIStatus=帧完成旗；**帧预算记账**（每域 update 记周期常数，自 Mesen trace 标定）决定超时掉帧 | ⚠ 确定性 tick 不能天然复现「主循环超时掉帧」——记账构件必须进 Phase 0（见对抗检验 6） |
| **OAM DMA** | `SpriteAllocator` + `VideoFrame{oam[64], nametables, palettes, scroll}` → `trait VideoSink` | ⚠ L4 像素 diff 需**软件 PPU 光栅器**复现 8-sprite/scanline 丢弃、优先级、sprite0（D4 保真）；否则 L4 只能降级为 OAM/nametable 状态 diff（需用户裁定） |
| **JumpEngine 指针表** | `enum` + `match`（MainRoutine/TitleRoutine/敌/门/电梯 handler） | 分派语义保真（索引值不变），指针表退役 |

### 改写策略

**改写单元 = 域内行为簇**（模块=域，簇=一组共享状态迁移的例程，如 samus 分：物理积分 / 形态状态机 / 受伤·血量 / 门·地形交互 4 簇）。每簇六步（继承 E9 流程）：

1. **定域取证**：读 asm（banner+E3/E4 管线+constants_ram 结构）→ 产出 `docs/spec/<module>.md`：行为规则清单 + 状态字段表 + 调用/共享 RAM 契约 + bank:addr 证据指针（最小集起步，迭代补全）。
2. **域结构**：struct+method，命名走 data/naming 台账（每条 rename 一条记录，A/B 级证据）。
3. **memmap 适配**：encode/decode ↔ RAM 域（地址只此层）。
4. **shim 适配**：域方法 → L2In/L2Out 写集（由状态变更推导，不手写地址）。
5. **测试**（见下）+ 6. **登记**：port-map registry + 矩阵格 + 独立复审（verifier 角色，证伪立场）。

**单元测试设计**（模块级，期望值一律来自 oracle 捕获，不手推）：

- **状态迁移测试**：域状态 fixture（Mesen2 savestate/RAM dump 抽取；docs/design/state-fixture-verification.md 方向）+ 输入序列 → 断言域状态迁移一致。
- **边界条件**：u8/i8 wrap、BCD 进位（AddHealth/SubtractHealth）、槽位耗尽（精灵游标/敌槽/爆炸槽）、卷轴/nametable 环绕、0 血/满血/能量罐、timer 到期跳转、bank 窗口边界。
- **异常路径**：NMIStatus 超时掉帧、门/电梯切换中断、死亡→爆炸→Game Over、暂停恢复。
- **突变矩阵**：每簇 ≥2 假想缺陷注入必须被捕获（E9 不变式）。

**留档追溯**：`data/naming/port-map.toml`（机器可读）：
`asm="bank7:CB29"` → `rust="game_core::world::update"` + evidence + commit + cluster id。
gate 工具核对 S 面分母 × registry **双射**；docs/spec 与 registry 同 commit 进库（追溯径 = registry + 台账 + git history）。

### 验证方案

三层裁决（有效性排序：L4 > L3 > L2）：

1. **模块逻辑一致**（同输入→同输出）：
   - **L2 函数级**（保留为安全网，不作改写单元）：写集/寄存器终值对拍（现有 l2_cases.jsonl 机制）。
   - **L3 模块级**：场景对拍——Mesen2 跑 N 帧（同一 savestate+输入序列）↔ Rust tick N 帧，按域抽 RAM 快照逐字节 diff（enemy 区 $0400-$04FF、Object 区 $0300-$03FF、RoomRAM、SpriteRAM $0200、声音区 $0600）；帧相位对齐。
2. **无遗漏**：S 面分母（tools/completeness_denoms.py）× port-map registry 双射核查（新工具 `tools/port_coverage.py`）——每个 asm 子程序恰有一个 Rust 归宿（语义 fn 或显式 `unported!` fail-loud 桩，爆在 bank:addr）；D 面数据表逐表登记。**对拍只裁对不对、不证全不全**（D7），全靠 registry 双射 + 矩阵左移。
3. **L4 帧对拍**（最终判据）：场景回放 → Mesen2 截图 vs 软件光栅输出像素 diff；白名单仅限已裁定视觉例外（用户确认）；基线 ratchet 只升不降。

---

## 对抗性检验

**5. 模块划分粒度**：三个候选中「逻辑功能域」正确，但必须显式加「簇」子层：
- 太细（子程序级=现状 521 个机械名 fn）：无组合语义，跨函数 bug 不可测，覆盖率是假象（27.3% 只证「翻了」不证「对了」）。
- 太粗（整域一 PR）：不可验证、失败定位成本爆炸。
- **判定：域（~12 个）= 模块，行为簇 = 改写单元，L2 函数级对拍=安全网**。asm 文件≠模块（door 散在 areas_common+prg7；敌驱动在 prg7、AI 在 enemies/）——边界由调用图/数据流定。结论：粒度设计正确但需防「以文件定边界」的偷懒回归。

**6. 6502 构造的 Rust 等价**：
- **零页**：分层映射（暂存→局部 / 持久→字段）正确且必须。反例实证：l4_ready_impl 的 SamusState 平铺 60+ 字段混装 enemy/render 状态=按地址序平铺的架构病。✓ 决策正确。
- **bank**：逻辑分组正确；风险=跨 bank thunk ABI（findings/cross-bank-thunk-abi.md 已有）与窗口读表（L2In.window_bank 已参数化）。✓ 可行。
- **NMI**：⚠ **最大风险**。asm 语义=主循环与 NMI 竞态（超时→掉帧，E4）。纯确定性 tick 复现不了掉帧；须**帧预算记账**（周期常数自 Mesen trace 标定），或把掉帧场景登记 BUG/白名单。不解决此点，L4 在重负载场景（多敌+爆炸）必然分叉。
- **OAM**：抽象可行，但 L4 像素级必须有软件 PPU 光栅器（8-sprite 丢弃是可见像素差，D4 要求保真）。成本高、须单列构件与工期；不做则 L4 判据降级（需用户裁定）。
- **判定：可行，但「帧预算记账 + 光栅器」必须列为 Phase 0 显式构件**，不是后续细节。

**7. 改写策略可执行性**：
- 单测：fixture 来自 oracle 捕获 + 突变矩阵 → 可执行、可判定；风险=fixture 维护/savestate 对齐成本（已有 state-fixture-verification.md 方向）。✓
- 留档：port-map registry 机器可查双射 → 可行，与 data/naming 纪律一致。✓
- 风险：spec-before-code 产出量大（12 域×簇）；缓解=spec 最小集（行为规则+证据指针）起步、随簇迭代。改写顺序建议先「叶子域」（render 数据通路、physics 积分）后「编排域」（world/samus 状态机），降低集成期分叉面。

**8. 验证方案充分性**：
- 函数级（L2）必要不充分：等价切片拼接 ≠ 等价程序（组合顺序/共享 RAM 契约/帧相位——恰是 D7 列的缺口类）。
- 模块级（L3）捕捉交互缺口最有效，但行为空间无机械分母——只能场景清单声明覆盖，绿≠完整（D7）。
- L4 像素=端到端最终判据，但同样只裁对错不证全。
- **结论：三层缺一不可**；缺口左移靠 registry 双射（S 面机械）+ unported! fail-loud + 首分歧分类路由（pipeline §4.2）。验证充分性取决于场景清单质量——Phase 0 须交付**场景分层清单**（每域≥2 域内场景 + 跨域契约场景（NMI 同步/共享 RAM/调用序）+ 端到端流程场景）。

## 可行性判定

**有条件可行**。条件（缺一即降级为不可行）：

1. **零页分层映射铁律**：$00-$11 暂存→函数局部/参数；禁止地址序平铺 struct（现有 SamusState 须按域拆分重构）。
2. **帧预算记账构件**进 Phase 0（掉帧语义；常数自 Mesen trace 标定）。
3. **软件 PPU 光栅器**单列构件（或用户裁定 L4 降级为 OAM/nametable 状态 diff + 白名单）。
4. **sound 独立验证面**：APU 寄存器流 diff（不进像素判据）。
5. **改写单元=域内行为簇** + port-map registry 双射门禁 + 独立复审。

**Phase 0 交付物清单**（可验证）：① 模块 DAG + 接口签名（本文 §模块划分）② UpdateWorld/NMI 顺序契约 spec（E3/E4）③ port-map registry v0 + `tools/port_coverage.py` 双射核查 ④ 零页映射规范（暂存/持久二分表）⑤ 帧预算记账方案（trace 标定方法）⑥ 光栅器/降级裁定项 ⑦ 场景分层清单 v0 ⑧ **2 个试点簇**（samus::physics::integrate_y（已有 int_y 证明级等价）+ world::room 加载）跑通六步流程，验证管线后进入批量改写。
