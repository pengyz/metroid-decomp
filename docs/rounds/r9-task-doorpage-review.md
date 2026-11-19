# r9 复核：门框页选修复（`593d9b45`）+ 子弹页选（`317fcb35`）

复核员：`reviewer-doorpage`（独立对抗复核，证伪立场；未改仓库，变异已复原）
复核对象：`593d9b45`（结构绘制门框页选）+ 其后的 `317fcb35`（子弹 BG 碰撞页选，源自本次复核 P1）
复核基线：`HEAD = 593d9b45`，worktree 干净（复核结束时非干净是并发房扫所致，非复核员产物）

## 1. 逐项判定

| 项 | 对象 | 判定 | 依据 |
|---|---|---|---|
| A | asm 事实（hi=名表号、MakeRoomRAMPtr 换算 `& $04`） | PASS | `GetNameTableAtScrollDir ; 07:EB85`（`prg7_engine.asm:9002-9008`）、`MakeRoomRAMPtr ; 07:E96A`（`:8516-8556`）、`GetObjRoomRAMPtr ; 07:D79F`（`:4789-4801`）、`LoadObjectPositionFromTemp ; 07:D638`（存 `hi & $01`） |
| B | 同族漏改（还有谁把 hi 0/1 当页选位） | **FAIL（P1）** | `weapon/projectile.rs:138-139` 直传 `Objects.hi` 当 `Room::addr` 页选位 ⇒ 子弹 BG 碰撞永远读 A 页；`bullet_move`（`:101-107`）漏 asm `:4540-4548` 的 hi 落写 |
| C | 测试有效性（变异） | PASS | 去 `<< 2` ⇒ 新测红（`hi=1 row=10 addr=0x6542 / left 0 / right 78`），对页负断言有效 |
| D | 实测可复现 | PASS | NT1/NT3 的 `$2542…$25E2`/`$2D42…$2DE2` 12 格两腿均 `0x4E`；全文件仅 22 B 差（20 B = HUD 十格，2 B = scrollX/ppuCtrl 相位） |
| E | 门框写入逐条对 asm | PASS | `WriteDoorBGTiles_Common ; $8CFD`（`areas_common.asm:1694-1735`）：y=$50 起 6 行、`DoorXTable` $E8/$10、页内 +$20k 不跨页、`DoorRoomRAMPtr,y` 字节索引 = `(x_off>>4)&3` |

## 2. P1 的处置（已修，`317fcb35`）

- 碰撞页选改走 `crate::enemy::motion::make_room_ram_ptr(mem, y, x, hi)`。
- `bullet_move` 重写为 asm 三件（`UpdateBullet_Move ; 07:D624` → `ApplySpeedToPosition ; 07:FD8F` → `LoadObjectPositionFromTemp ; 07:D638`），失败即 `status ← 0` 且 `hi = phi & $01`。
- 新增 3 条锚测（`weapon/projectile.rs` 的 `mod tests_bullet_hi_page`），变异验证：页选位钉 0 ⇒ 首条红（`left: 1 / right: 4`）。
- 矩阵 `run-all --skip-emu` **match 151→154 / diff 9→6 / blocked 0**（160 格，逐格无回退）。

## 3. 后续复核（`reviewer-bullet`）追加的 A5（P1，已修 `3d45fc3e`）

asm 失败径 `CheckBulletStat:4304 bcc @collided` → `@collided:4315 lda status / beq Lx069` **同时跳过** `dec SamusProjectileDieDelay,x` 与 `jsr AnimDrawObject`；`317fcb35` 的旧形仍 `dec base+0x0F` 并无条件 `anim_draw_object`。已改为 `bullet_move` 返回 `bool`、`update_bullet` 记 `deleted` 并连带跳过绘制。

## 4. 未处置（P3，如实留档）

- **P3-1**：`593d9b45` 的 commit message 写「`cargo test --workspace` 683」，实为 **lib** 计数（当时 workspace 基线 774）。message-only amend 会被 F② 闸拒（无语义行），故不改写，在此更正。
- **P3-2**：`data/status/nes_map.json` 中 `world/room_draw.rs:324→325` 的更新不忠实——本次改动使该函数整体下移 6 行，等价锚应为 **330**；现 325 落在语句行（`set_is_hit(0)`），门禁只拒空行/纯注释故通过。该 span 是 `$0685-$07FC`（栈页区），其 refs 本身是「谁碰过该区」的示例性指针，无单一忠实行；留待下次动该文件时一并校正。
- **P3-3**：复核员未独立复跑 room0 房扫 / `run-all` / 全量测试（限时），如实标注为未复现项。

## 5. 复核员补充事实（有价值，已收）

- `crates/game-core/src/world/room_ram.rs:45-54` 的 `offset()` 第三参确为页选位；`pickup/spawn.rs:181-186`、`enemy/motion.rs:549-554`、`samus/physics.rs:131-134`/`:1381-1383` 是**内联同式**（不是调 `make_room_ram_ptr`），只有敌人 NPC 走那个函数。
- 并发房扫持 `analysis/.l4_run.lock` 时 `run-all` 不可跑；`cases/_tmp_sweep_*` 会被 `tools/l4_diff_run.py:1101` 的 glob 纳入分母（实测 160→232），故临时用例必须移出 `cases/`。
