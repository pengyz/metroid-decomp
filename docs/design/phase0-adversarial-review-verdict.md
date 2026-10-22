# Phase 0 架构设计交付件·对抗复核 verdict

> 2026-10-22 · 复核任务（四项核（对抗复核核（证伪立场（独立重导证据（正确性核））））·
> Evidence A 级（bank:addr 引证）Confidence 95

复核对象 7/8 交付件：模块划分（333d965）/UpdateWorld·NMI 契约（fe528af）/改写策略+验证方案
（080a74d）/port-map registry（06cdfd9）/零页二分表+帧预算（1715aa7）/场景分层 v0（5535af1）/
光栅器裁定项（465aa28）。

## 逐项 verdict（证伪立场·独立重导）

| # | 问题 | verdict | 独立重导证据 |
|---|---|---|---|
| ① | 模块划分合理？ | **pass** | 域级模块×逻辑单元+DAG+接口签名（tick/SpriteAllocator/EnemyAi/World/ApuFrame）——与两层架构（game-core 纯逻辑/game-data 数据表）红线一致 |
| ② | asm→Rust 对应正确？ | **pass·带更正注** | NMI 两相独立重导=prg7_engine.asm:249-280（OAMDMA→NMIStatus 门控→NMIScreenWrite→…→sty NMIStatus）与契约 §1.2 一致；**Temp04「三义同址」欠实**——独立重导 constants_ram.asm:255-340 = **≥8 别名同址 $04**（YSlotRadiusY/SpeedY/RoomRAMPtr/MetaspriteFlipFlags/NumBlocksToCheck/ControlBits/Displacement/…）（暂存→局部映射决策方向对，注记应改「≥8 义」） |
| ③ | 改写策略+验证可执行？ | **pass** | 六步流程+L2/L3/L4+S 面×registry 双射——与 080a74d/06cdfd9 自证一致（SamusJump 07:D002 组合法实证） |
| ④ | registry+port_coverage 可行？ | **pass** | 双射核算法（S−registry=遗漏 FAIL/registry−S=多余 FAIL）机械可导；label 类型辨（1321/431/50 计数实录）可解析 |
| ⑤ | 零页二分表+帧预算正确？ | **pass·带风险注** | Temp04 多义实证（上行）；NMIStatus 掉帧语义（:251-252 bne gate+主循环超时跳 vblank 后半）=**最大风险点**（帧预算记账 NMI/主循环分账是对症设计） |
| ⑥ | 场景分层充分？ | **pass** | 每域≥2 域内+跨域契约（NMI 同步/共享 RAM/调用序）+端到端流程（S1-S5）——v0 充分（增量可扩） |
| ⑦ | 光栅器裁定项充分？ | **pass** | 两案成本/收益/风险齐（案 A 高成本伪 diff 风险 vs 案 B 低成本像素漏判风险）+建议案 B+一次性像素终检；三择项留用户裁定（处置正确） |

## 总 verdict

**pass（带 1 处更正注记+1 处风险注记）**——7 件可执行；更正注记=Temp04 别名数（三义→≥8 义，
原地记录不删原文（证伪更正文化））；风险注记=NMI 掉帧=最大风险（帧预算分账守恒）。

Evidence: constants_ram.asm:255-340（Temp04 ≥8 别名 $04）+ prg7_engine.asm:249-280（NMI 两相）
——独立重导（bank:addr 引证）。
Confidence: 95
