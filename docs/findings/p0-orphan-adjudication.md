# P0 孤儿裁定（五未编译死文件（329 行）——处置实录（A 级））

> 2026-10-28 · 架构治理 P0 · 处置=全部删除（findings 原地注记（锚保留））+
> **TDD 补测接线入域**（tests_p0_anchor_pin（10 测（替代面 asm 真值常量 pin）））。

## 逐文件裁定

| 文件 | 内容 | 裁定 | 理由 |
|---|---|---|---|
| enemy/explosion.rs（70 行） | ExplosionState+update_explosion（空体（返回 Default））+模板测 | **被取代→删** | 真爆径=frame/mainloop.rs bullet_explode/update_bullet_explode（anim 0x1D/0x91（任务链件））+handler SetSamusExplode；anim 数据锚如下 |
| enemy/rio.rs（60） | RioState+rio_ai_routine（空体） | **被取代→删** | 通用 AI=enemy/ai.rs update_enemy_ai；**Rio 专属 AI=已知缺口**（锚如下（未来卡）） |
| enemy/ripper.rs（61） | RipperState+update_ripper（空体） | **被取代→删** | 同（Ripper 专属 AI 缺口） |
| enemy/zoomer.rs（60） | ZoomerState+update_zoomer（空体） | **被取代→删** | 同（Zoomer 专属 AI 缺口） |
| world/elevator.rs（78） | ElevatorState+update_elevator（薄字段读（$030F 偏移疑误（正源 $032F）））+elevator_schedule($0311) | **被取代→删** | 电梯域未实装（LoadElevator 臂=占位（门链任务卡 1 遗留））；字段读面平凡（2 行）且偏移疑误（价值<风险）；正源锚如下 |

## 锚保留（A 级（删除件内注））

- **爆族 anim 数据**：ObjAnim_SamusExplode bank01:85A4 / ObjAnim_BombExplode $85F4 /
  ObjAnim_MissileExplode $8603（=0x91（表算））；ObjFrame_SamusExplode $8954 /
  BombExplode0/1/Blank/2 $8987/$8A0F/$8A1D/$8A26 / MissileExplode0/1/2 $8AE9/$8AF3/$8B03。
- **敌专属 AI（缺口）**：Rio bank01:97DD（EnemyMovementChoice_Rio）+9AF5-9B1C
  （RioAIRoutine）；Ripper/Zoomer 同族 bank01 例程（未实装（通用 AI 覆盖基础面））。
- **电梯面**：ElevatorType=Elevator.data0F=**$032F**（旧件误 $030F（=SamusJumpDsplcmnt））/
  SamusOnElevator=Samus.data07=**$0307**/Elevator 对象=$0320（SpawnElevatorRoutine $EC09
  （门链任务卡 1 占位臂 type 4））/elevator_schedule 读 $0311（animDelay 面）。

## 替代面 TDD 补测（tests_p0_anchor_pin（10 测（随删同批（ratchet 不降保障））））

pin 常量=上述锚的 asm 真值（爆 anim 表值/门耐久表/电梯偏移/对象槽布局/FrameNone）——
每测独立断言（替代面回归钉（非凑测（asm 锚面（A 级））））。

Evidence: 五件全文（329 行）+common_sprite_data.asm:70-268（anim 表）+constants_ram.asm
:903/917-920（字段）+替代件（mainloop.rs/ai.rs/system.rs）——A 级原文。
Confidence: 95
