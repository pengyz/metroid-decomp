# 炸弹子系统 / 瓦片爆池 / 按区 VRAM 字符串段（2026-11-20）

## 1. 为什么做这三件

TAS `any%` 6725M 的主路径必用炸弹（破可炸瓦、破 Norfair 泡、拆 Metroid），而
`crates/game-core/src/weapon/projectile.rs` 的对象状态 dispatch 对 `5..10`（炸弹三态 ×
两套半程）是**记账 no-op**（`DIVERGENCES.push(0xD4CB)`）⇒ 实机按 B 毫无反应。
炸弹炸瓦又依赖 `TileBlasts`（`$0500`）池与**按区**的碎块图案，两者此前都不存在。

## 2. asm 锚（bank7，全部 A 级逐行读过）

| 例程 | 地址 | 关键语义 |
|---|---|---|
| `DoOneSamusProjectile` | `07:D4CB` | 12 项跳表：`5/8→BombInit`、`6/9→BombCountdown`、`7/10→BombExplode` |
| `CheckBombLaunch` | `07:D150` | Gear bit1 ∧ FIRE ∧ `speedY==0 && onElevator==0`；槽 D/E/F 找空；`y+4`；`status←wa_LayBomb(8)` |
| `BombInit` | `07:D65E` | `anim←BombTick($7F)`、`dieDelay←$18`、`inc status`、`DrawBomb`（每 3 帧一换） |
| `BombCountdown` | `07:D670` | `FrameCount lsr / bcc` ⇒ **只在奇数帧**递减；归零 ⇒ 切 `BombExplode($82)`（另一套半程用 `SamusRunPntUp($37)`，原版原样）、`inc status`、`SFX_BombExplode` |
| `BombExplode` | `07:D691` | `inc dieDelay` → 相位炸瓦 → `animFrame==$F7` 删槽 |
| `BombExplosion_CollisionWithBG` | `07:D6A7` | 五相位：1 中心 / 2 上两行 / 3 下两行 / 4 左两格 / 5 右两格；越界到属性表时按滚动方向退出或跳对面名表 |
| `LD77F` / `LD78B` / `LD798` | `07:D77F` / `D78B` / `D798` | 16 位加减后 `and #$07 / ora #$60` 回到 RoomRAM 高字节 |
| `BombCurrentTile` | `07:D76A` | `CheckBlastTile` 阈值门 → `< $A0` → `IsBlastTile@SkipCheck…`（**跳过 `UpdatingSamusProjectile` 门**，炸弹恒按弹径炸瓦） |
| `UpdateAllTileBlasts` / `UpdateTileBlast` | `07:FE1B` / `FE27` | 6 项状态表；`$FE` 串尾走 `pla/pla/rts`（**双弹返回**，跳过余下槽） |
| `DrawTileBlast` | `07:FEDC` | 帧串写 RoomRAM（行距 `$20`）+ 转名表地址排 PPU 缓冲 |
| `UpdateTileBlastAnim` | `07:FF54` | 每 2 帧换串项；缓冲满则回滚游标（asm 自承是缺陷，保真） |
| `UpdateTileBlast_Respawned` | `07:FE83` | 元 tile 中心 `ptr+$21`、两次 `lsr/ror` 换算；压到 Samus ⇒ 伤害 `$50` |
| `GetVRAMStringPtr` | `07:FECC` | `lda VRAMStringPtrTable,y` —— y 是**字符串 id**（不是 VRAM 地址） |

## 3. 数据面：`VRAMStringPtrTable ; 各 bank $97AF`（17 项 u16）

按区实测（ROM 直读，`prg = bank*0x4000+(addr-0x8000)`）：

| 区 | 表起点（首个串） | 真串数 | 表尾别名 |
|---|---|---|---|
| Brinstar (bank1) | `$9D36` | **10**（id0..9） | id10..16 全指 `$9D6A`（= `EnAnimTable` 基址，头字节 `$00`） |
| Norfair (bank2) | `$9B9E` | 12 | id12..16 → `$9BDA` |
| Tourian (bank3) | `$A32B` | **17（全真）** | 无 |
| Kraid (bank4) | `$9C4A` | 12 | id12..16 → `$9C86` |
| Ridley (bank5) | `$9B49` | 12 | id12..16 → `$9B85` |

- 串体连续：`ptr[i+1] == ptr[i] + (1 + height*width)`，尺寸由头字节 `%hhhhwwww` 给出。
  提取侧把这三条（头半字节非 0 / 单调查 / 首尾相接）作为**机械自检**，不自洽就拒绝出包。
- 表尾别名折叠为**零长**（相对偏移 = 串体长 ⇒ `assets::vram_string` 返回空切片）。
  原版对越界 id 会读到动画表的字节（`Brinstar` 的 `$74` 用 id `$0A` 就是这条）——
  行为上等于画出乱图，但 `Brinstar` 没有 `$74` 这种瓦，实际不可达。
- 包内段：`area.{br,no,to,kr,ri}.vram_strings`（id 191–195），布局
  `17 × u16（相对串体起点偏移） + 串体`；长度分别 86 / 94 / 94 / 253 / 94 B。

## 4. 我改了什么

- `crates/game-data/src/{pack,extract,assets}.rs`：新段 + `assets::vram_string(area,id)`。
- `crates/game-core/src/weapon/bomb.rs`（新）：`check_bomb_launch` / `bomb_init` /
  `bomb_countdown` / `bomb_explode` / 五相位炸瓦 / `draw_bomb`。
- `crates/game-core/src/world/tile_blast.rs`（新）：13 槽状态机 + 动画串 + 绘制 + 复原伤害。
- 接线：`projectile.rs` 的 `5..10` 分支、`world/update.rs::uw_tile_blasts`、
  `samus/handler.rs` 的 `CheckBombLaunch`（原 `DIVERGENCES.push(0x0D150)`）。
- 收口纪律：新文件**零裸字节面**——池内读写走 `frame/memory.rs` 的 `tb_*` 访问器
  （与 `frame_count()`/`kraid_lint_counter()` 同族豁免），对象/萨姆斯走 typed 句柄。

## 5. 顺带修的既有错（不在本卡范围，但影响 Tourian 结局链）

`crates/game-core/src/frame/tick.rs` 的死亡收尾把 **`$687B`（`KraidStatueStatus`）** 当
`MotherBrainStatus`。asm `GameEngine ; 07:C92A`（`prg7_engine.asm:2063-2080`）读的是
`MotherBrainStatus ; $0098`（`constants_ram.asm:708`）。用错址 ⇒「MB 已死 ⇒ 立即
GameOver」这条边**永不触发**，进 Tourian 死亡必走 40 帧计时器。
现补 `ram_map::MOTHER_BRAIN_STATUS` + 双向分支测（`$0098=#$0A` 走立即分支；
`$687B=#$0A` 不得影响）。

## 6. 验收与诚实边界

- `cargo test --workspace` **916**（炸弹 10 条 + 瓦片爆 8 条 + 分支 1 条）。
- 变异验证：`y+4` 去掉 ⇒ 埋弹位置测红；`tile >= $98` 门去掉 ⇒ 该测红；
  `$0098→$687B` 回退 ⇒ 分支测红。三条均已复原。
- `gates check` PASS=23；L4 矩阵 **154/6/0**（160 格）与基线逐格一致
  ——**现有 20 个用例没有一格按过 B**，故本卡无像素证据，属覆盖缺口，
  不是「改对了」的证明。炸弹的像素级验证要等 TAS `any%` 适配跑。

## 7. 已知未做

- 炸弹对敌/对 Samus 的伤害与击退：`uw_collision_detection` 已按弹型 `wa_BombExplode`
  分支，但炸弹槽走的是对象池而非敌池，二者交叉面尚未逐条对齐 asm `:10096/:10161/:10228`。
- 瓦片爆复原压在 Samus 上的**击退分量**未复刻（`DIVERGENCES.push(0x0FE94)`），
  伤害与 `isHit` 是全真形。
- `TileBlast` 的 `spare05` 字段语义：原版把 `animIndex` 镜一份进去，`UpdateTileBlastAnim`
  回滚时用——已复刻取值，但其「为何要镜像」的动机未考证（C 级）。
