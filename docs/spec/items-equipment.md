# 道具 / 能力 / 门禁规格（B8）

> 目标：把「拿到道具 → 获得能力 → 能开哪些门、破哪些砖、受什么限制」的 ROM 语义写成可实现规格。
> 只写本文档，不改代码。证据级：**A**=字节/指令级可复算；**B**=调用图/数据流推断；**C**=假设（只进 §6）。
> 交叉引用：[`room-data-format.md`](room-data-format.md)（房间与特殊物品表）、[`doors-elevators.md`](doors-elevators.md)（门记录/转移链）、[`room-structures.md`](room-structures.md)（结构记录）。

## §0 一句话结论

道具只有两个持久化面：**能力位** `SamusGear=$6878`（8 个 bit）与**已拾取物品历史** `NumberOfUniqueItems=$6886` + `UniqueItemHistory=$6887-$68FC`（每件 2 字节 ID）。门禁只有三种：**导弹门按命中次数扣 HP**（红 5 发、十发门 10 发，非导弹弹开）、**可爆破砖按 tile id 范围**、**能力位直接读**（变形球/高跳/ varia / 冰冻/波束/长束/旋转攻击）。

> 【对卡面的更正】卡面写的「道具区状态位（`$687F` 一类）」不成立：`$687F` 是 `SamusAge+2`（游戏用时最高字节，`constants_ram.asm:1237-1240`）。道具持久化真源是 `$6886/$6887`。另：`$69B4-$6A73` 的 `ItemHistory` 在 `constants_ram.asm:1287` 被注明 `not used`，且密码交换用的是 `PasswordByte $6988-$6999`（另一套位图）。

## §1 道具类型表（`pu_*` → 效果 → 写点）

`.enumid` 定义在 `constants.asm:54-65`，取值与 `SamusGear` 位一一对齐（`gr_*`，`constants.asm:69-77`）：

| pu id | 名称 | 效果 | 能力位 / 计数器写点 | Rust 现状 |
|---|---|---|---|---|
| `$00` | `pu_BOMBS` | 炸弹（下+跳） | `SamusGear |= $01`（`MakeBitMask 07:DB2F` + `ora/sta $6878`，`prg7_engine.asm:5636-5644`） | 已端口 `samus/handler.rs` |
| `$01` | `pu_HIGHJUMP` | 高跳：重力 `$18→$12` | 同上 `|= $02`；读点 `LCFC5` 族（`:3466-3473`） | 已端口 `handler.rs:563` |
| `$02` | `pu_LONGBEAM` | 长束（弹速/距离） | `|= $04` | 已端口 `weapon/projectile.rs` |
| `$03` | `pu_SCREWATTACK` | 旋转攻击（旋跳中撞死敌） | `|= $08`；判定 `IsScrewAttackActive 07:CD9C`（`:3014-3044`） | 已端口 `physics.rs:1083-1096` |
| `$04` | `pu_MARUMARI` | 变形球 | `|= $10`；`SetSamusRoll 07:D0B5`（`:3591-3610`） | 已端口 `handler.rs:997-1060` |
| `$05` | `pu_VARIA` | 变体服：受伤减半 | `|= $20`；读点 `SubtractHealth 07:CEBF`（`:3230-3245`，`lsr` 两次 + 进位补 `$4F`） | 已端口 `samus/state.rs:1014-1015` |
| `$06` | `pu_WAVEBEAM` | 波束（穿墙） | **先清** `$C0`（冰+波）再 `|= $40`（`:5633-5644`） | 已端口 `pickup/spawn.rs:261-263` |
| `$07` | `pu_ICEBEAM` | 冰冻束 | 同上，清 `$C0` 后 `|= $80` | 同上 |
| `$08` | `pu_ENERGYTANK` | 能量罐：罐数 +1（上限 6）、满血 | `EnergyTankCount $6877 = min(6, n+1)`；`Health+1 $0107 = (n<<4)|$09`；`Health $0106 = $99`（`:5674-5691`） | **地址错位（见 §4）** |
| `$09` | `pu_MISSILES` | 导弹容量 +5（同时补满当前数） | `AddToMaxMissiles 07:DD97`：`MissileCount $6879 += A`、`MaxMissiles $687A += A`，各自溢出钳 `$FF`（`:6075-6094`） | **地址错位（见 §4）** |
| `$0A` | `pu_MISSILEDOOR` | **不是可拾取道具**：红/十发门开启后写进物品历史的「伪类型」 | 开门时 `Temp09_ItemType=$0A` + `MapScrollRoutine→CreateItemID→AddItemToHistory`（`areas_common.asm:1540-1560`） | 已端口 |

要点（A）：

- 能力位的掩码就是 `1 << pu_id`，由 `MakeBitMask`（`07:DB2F`：`sec/lda #$00/rol/dey/bpl`）算出，零号表不参与。
- 两把束武器是**互斥**的：拾取冰/波前先 `and #~($C0)`，所以后拿的替换先拿的（`prg7_engine.asm:5633-5639`）。
- 能量罐上限 `6`（`CMP #$06/BEQ` 不再加罐，但**仍然满血**）；满血写法是 `Health=$99`、`Health+1=($0?<<4)|$09`，即 6 罐=999、5 罐=899…。
- 导弹容量和当前数一起加，所以拾取即补满 5 发；上限 255。

### §1.5 道具在房间数据里的位置（每区特殊物品表，A）

特殊物品表由 `SpecItmsTblPtr`（各区 `?…:9598`）指向，`ScanForItems 07:ED98` 按 **Y 行** 沿链表扫（`[y][nextLo][nextHi]`，Y 不匹配就走 next 指针），命中行再按 `[x][delta|$FF][type,sub,param][$00]` 取条目——记录布局见 [`room-data-format.md`](room-data-format.md) §1.5。

| 区 | 指针 | 表基址 | 出处 |
|---|---|---|---|
| Brinstar | `01:9598` | `$A3D6` | `prg1_brinstar.asm:85-86` |
| Norfair | `02:9598` | `$A2D9` | `prg2_norfair.asm:83-84` |
| Tourian | `03:9598` | `$A83B` | `prg3_tourian.asm:83-84` |
| Kraid | `04:9598` | `$A26D` | `prg4_kraid.asm:89-90` |
| Ridley | `05:9598` | `$A20D` | `prg5_ridley.asm:91-92` |

条目 `type` 字段就是 `it_*`/`pu_*`：`it_PowerUp($2)` 走 `SpawnPowerUp`（本卡主线）；`it_Elevator($4)`/`it_MotherBrain($6)`/`it_Zebetite($7)` 等各有专属 spawn（见 §2 表）。

## §2 获取链（拾取 → 历史 → 授予 → HUD）

调用链（A，xref 已核）：

```
SpawnPowerUp 07:EDFE        ; 房间特殊物品表出栈 → 找空 PowerUp 槽
  PrepareItemID 07:EE3D     ; (type, itemX, itemY) → CreateItemID 07:DC67
  CheckForItem 07:EE4A      ; 命中 NumberOfUniqueItems/UniqueItemHistory ⇒ 已拿过，不生成
UpdateAllPowerUps 07:DB37   ; 每帧两槽（$0340/$0348）
  UpdateOnePowerUp 07:DB42  ; 描画（ObjDrawFrame），未画出/未触碰即退
  AreObjectsTouching 07:DC7F; 仅屏距判（不含 hi 位）
  → 拾取：PowerUpMusic → GetItemXYPos 07:DC1C → AddItemToHistory 07:DC54
  → 按 type 分支授予（§1）→ PowerUpDelayFlag=$FF → 槽清零 → SelectSamusPalette
DisplayHUD 07:E0C1          ; 每帧重画状态栏（在 HP/弹药变化后同一帧可见）
```

RAM 字段（A，`constants_ram.asm:1230-1254`）：

| 地址 | 名字 | 语义 |
|---|---|---|
| `$6877` | `EnergyTankCount` | 能量罐数 |
| `$6878` | `SamusGear` | 能力位 |
| `$6879` | `MissileCount` | 当前导弹 |
| `$687A` | `MaxMissiles` | 导弹上限 |
| `$6886` | `NumberOfUniqueItems` | 历史字节数（每件 +2） |
| `$6887-$68FC` | `UniqueItemHistory` | 每件 2 字节 ID：高字节=类型（含 `it_*`/`pu_*`/`ui_*`），低字节=坐标编码 |

物品 ID 编码（A）：`IIIIIIXX XXXYYYYY`——高 6 位是类型（`pu_*`/`it_*`/`ui_*`），低位 5 位是地图 Y，中间是地图 X；`CreateItemID 07:DC67` 生成、`CheckForItem 07:EE4A` 线性比对（先比高字节、再比低字节，`dey/dey/bne`）。

`CheckForItem` 的**四个读点** + `AddItemToHistory` 的**两个写点**（A，各语义不同）：

| 调用点 | 用途 |
|---|---|
| `prg7_engine.asm:9544`（`SpawnPowerUp`） | 普通道具：拿过就不再生成 |
| `prg7_engine.asm:9069`（`SpawnDoor` 族） | **导弹门**：拿 `pu_MISSILEDOOR` + 门坐标查历史，开过的门直接进 Open 态、不再要导弹 |
| `prg7_engine.asm:9689`（`SpawnMotherBrain`） | 母脑已死（`ui_MOTHERBRAIN`）⇒ 直接置 `MotherBrainStatus=8` 重画时间炸弹提示 |
| `prg7_engine.asm:9710`（`SpawnZebetite`） | 已打掉的 Zebetite（`ui_ZEBETITE1+n`）⇒ `status=$81/qtyHits=7` 以死体生成 |
| `prg7_engine.asm:12796`（`UpdateTourianItems`） | **写**：逃逸计时第一帧（`EndTimer+1==$99` 且 `EndTimer==0`）把 `ui_MOTHERBRAIN`（`$E0`/低位0）写入历史 |
| `prg7_engine.asm:12826`（`CheckZebetite`） | **写**：Zebetite `status==2→3` 时把 `ui_ZEBETITE1+n`（`$E4..$E8`，低位0）写入历史 |

HUD（A，`DisplayHUD 07:E0C1`，`:6710-6800`）：先铺 10 个 4 字节模板（`HUDTemplateDataTbl 07:E1B9`），再写 2 位 HP（`Health+1&$0F`、`Health>>4`）、3 位导弹（`HexToDec`，`MaxMissiles==0` 时擦掉导弹图标）；逃逸计时非零时导弹位改画 3 位倒计时。

## §3 门禁关系（能力 → 能开什么）

### §3.1 门：按命中次数扣 HP，只有导弹能伤红门

`DoorHitPointTable 07:8BD1 = 05 01 0A 01`（红 / 蓝 / 十发 / 蓝换音乐），初始化时写进 `DoorHitPoints`（`areas_common.asm:1445-1450`）。

命中判定 `SamusProjectileHitDoorOrStatue 07:E81E`（`:8240-8278`）三段：

1. 门的**房间指针**（`DoorRoomRAMPtr` 四列）与弹的 tile 指针 `(+1)&$04==0` 且 `&$1F==0` ⇒ 同一格；
2. 蓝门（`DoorType&1`）任意弹都算命中；红/十发门只认 `wa_Missile($0B)`，或 `wa_BulletExplode($04)` 且 `animResetIndex==$91`（导弹爆炸帧，`ObjAnim_MissileExplode`）——**其他武器弹开、只放 `SFX_Metal`，不扣 HP**（A）；
3. 命中写 `isHit bit2`（`$04`）；`UpdateDoor_Closed` 每帧 `dec DoorHitPoints`（`areas_common.asm:1486`），减到 0 开门；开门后 `lda #$50 / sta DoorHitPoints`（`:1506-1507`）作 80×2 帧的重关延迟，之后每 2 帧 `dec`（`:1576`）直到 0 关门。
4. 红/十发门**开门即进历史**（`Temp09=$0A` + 门坐标，`:1540-1560`）并且 `status=0` 移除——不会再关上；蓝门不写历史，80×2 帧后重关。

所以「导弹门」实为命中次数门：红 5 发导弹、十发门 10 发导弹；蓝门 1 发任意弹。导弹本身的扣弹在 `FireSamusProjectile` 族里（`MissileCount--`），不在门代码里。

### §3.2 可爆破砖：按 tile id 范围，任何弹/炸弹

`CheckBlastTile 07:D651`：Brinstar 可炸 `$80-$97`，其他区（含 Tourian）`$70-$97`；`$98-$9F` 与 `$A0+`（空气）都不可炸。命中后 `SetTileBlast` 生成 TileBlast 对象逐块消失。

炸弹走 `BombExplosion_CollisionWithBG 07:D6A7`：按 `SamusProjectileDieDelay` 的 1→5 阶段依次炸中心、上 2、下 2、左 2、右 2 共 5 格（`:4614-4780`），每格都按同一 `CheckBlastTile` 判。

**A 级负面结论**：`gr_SCREWATTACK` 在 ROM 里只有两处读点——`IsScrewAttackActive`（判定可用，`:3014`）与 `SamusJump` 起跳音效（`:3452`），**没有任何「旋转攻击破砖」的代码路径**；它只在与敌人接触时秒杀（`CollisionDetectionEnemy_ReactToCollisionWithSamus 07:F282`）。所以「螺丝砖」在本作不存在，卡面这一条按否证处理。

### §3.3 其余能力门禁

| 能力 | 机制 | A/B 证据 |
|---|---|---|
| 变形球 | `SetSamusRoll 07:D0B5` 读 `gr_MARUMARI`，无则回站立；窄通道靠**碰撞半径**变小实现，不靠门 | A（`:3591-3610` / Rust `handler.rs:997`） |
| 高跳 | `SamusJump` 起跳置 `SamusAccelY=$18`，有 `gr_HIGHJUMP` 改 `$12`（`:3466-3473`） | A |
| Varia | `SubtractHealth` 里 `lsr` 两次（半伤），进位补 `$4F` 保证十进制 | A（`:3230-3245`） |
| 冰冻束 | `gr_ICEBEAM` 决定弹是否带冻结位；冻结敌由 `EnsExtra.status=enemyStatus_Frozen` 处理 | A（Rust `weapon/projectile.rs`） |
| 波束 | `UpdateWaveBullet 07:D52C` **每步都调用** `UpdateBullet_CollisionWithBG`（`07:D580`，`prg7_engine.asm:4393`）；`bcs Lx074` 只在**没撞**时跳过移动，撞了仍 `jsr UpdateBullet_Move`（注释原文「move bullet even if collided」）⇒ 撞不可炸固实瓦时**穿过且不爆**；可炸瓦（`$70/$80-$97`）走 `IsBlastTile` 照常破砖再继续飞。真正引爆只发生在「打到敌人」（`UpdateBullet_ExplodeIfHitSprite`）或「Move 判出屏」（`ApplySpeedToPosition` 的 CF=0 ⇒ `CheckBulletStat` 走 `@collided`） | A |
| 长束 | `CheckBulletStat 07:D4F8`：有 `gr_LONGBEAM` 时**跳过** `dec SamusProjectileDieDelay` ⇒ 弹不超时；另在发射时把 `HasBeamSFX` bit7 置位（改音效） | A |

### §3.4 门禁速查（能力 → 挡住什么）

| 卡住玩家的东西 | ROM 判据 | 需要的道具 |
|---|---|---|
| 红门 | `DoorHitPoints=5`，只吃导弹（`wa_Missile` / 导弹爆炸帧 `$91`） | 5 发导弹 |
| 十发门 | `DoorHitPoints=$0A`，同上 | 10 发导弹 |
| 蓝门 / 蓝门换音乐 | `DoorHitPoints=1`，任意弹 | 无 |
| 可炸砖（`$70-$97`） | `CheckBlastTile` + `SetTileBlast` | 任意弹或炸弹（炸弹炸 5 格） |
| 窄通道 | 碰撞半径（变形球后更小） | 变形球 |
| 高台/长距离跳 | 重力 `$18→$12` | 高跳 |
| 热区岩浆（伤害 2 倍体感） | `SubtractHealth` 里 `lsr` 半伤 | Varia |
| 冻结敌人 | 冰束命中写冻结状态 | 冰束 |
| 隔墙打敌 | `UpdateWaveBullet 07:D52C`：撞固实瓦后 `jsr UpdateBullet_Move`（`07:D580-07:D586`，注释「move bullet even if collided」）⇒ 命中判定只在敌人/出屏 | 波束 |
| 旋跳撞死敌 | `IsScrewAttackActive`（仅 run/somersault 状态 + 上升中） | 旋转攻击 |
| Boss 进度 | `$687B/$687C` 雕像位（见 [`game-progress.md`](game-progress.md)） | 击败 Kraid/Ridley |

## §4 与 Rust 现有实现的差异表

| 项 | Rust 位置 | 结论 |
|---|---|---|
| 道具生成/去重 | `pickup/spawn.rs:38-160`（`scan_for_items`/`create_item_id`/`check_for_item`/`record_item`） | 一致（ID 编码、线性比对、`NumberOfUniqueItems+=2`） |
| 拾取描画 | `pickup/spawn.rs:174-240`、`pickup/powerup.rs:35-64` | 一致（含 `$5B SpritePagePos` 前后比、隔帧变色） |
| 触碰判定 | `pickup/spawn.rs:306-340`（`AreObjectsTouching` 屏距径） | 一致 |
| 能力件授予 | `pickup/spawn.rs:258-290` | **一致**（`MakeBitMask`=`1<<type`；冰/波先清 `$C0`） |
| 导弹罐 | `pickup/spawn.rs:268-272` | **错址 + 漏写**：读 `0x6877`（实为 `EnergyTankCount`）当上限、写回 `0x6877`；ROM 应同时加 `MissileCount $6879` 与 `MaxMissiles $687A`（`AddToMaxMissiles 07:DD97`），溢出钳 `$FF` |
| 能量罐 | `pickup/spawn.rs:274-283` | **错址**：读 `0x6874`（ROM 未定义该字节；`EnergyTankCount=$6877`）；`Health+1=$0107`、`Health=$0106` 正确 |
| 门 HP / 导弹门 | `world/room_draw.rs:335`（`DOOR_HP_TABLE=[5,1,10,1]`）、`weapon/projectile.rs:10-50` | 一致（含「非导弹弹开」与 `$91` 爆炸帧判定） |
| 红门进历史 | `world/room_draw.rs`（开门路径写 `DoorHitPoints=$50`） | **待核对**：未见 `Temp09=$0A` + `MapScrollRoutine` 的等价写（`grep pu_MISSILEDOOR` 零命中） |
| HUD | `DisplayHUD 07:E0C1` | **未端口**（`grep -rn "DisplayHUD\|HUD_TEMPLATE" crates/game-core/src` 零命中） |
| 爆破砖 | `weapon/projectile.rs`（BG 碰撞）、`world/tile_blast.rs` | 一致（tile 范围判据） |
| 导弹扣弹 | `samus/handler.rs:397-411`（`wrapping_sub(1)` + 归零后 toggle 递减） | 一致（连 `MissileCount=0` 时无守卫、会回绕的原版形也保真） |
| **波束弹道** | `weapon/projectile.rs:71-76`：status `2\|3` 直接复用 `update_bullet`（含 `UpdateBullet_CollisionWithBG`） | **语义不一致**：`update_bullet` 对固实瓦一律置 `collided=true` 并 `bullet_explode`（`projectile.rs:242-285`）；ROM 的波束撞固实瓦后**仍移动**（`07:D580` 的 `jsr UpdateBullet_Move`）、只有可炸瓦破砖，不会自爆。该分支已记 `DIVERGENCES 0xD522` 但行为仍错 |
| 旋转攻击 | `samus/physics.rs:1083-1096`（`is_screw_attack_active`） | 一致（只杀敌、不破砖） |
| 变形球 / 高跳 / Varia | `handler.rs:997-1060`、`handler.rs:561-565`、`samus/state.rs:1014` | 一致 |

## §5 复现命令与实跑输出

```bash
cd /home/peng/workspace/metroid-decomp
# 1) 门 HP 表（07:8BD1）
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=7*0x4000+(0x8BD1-0xC000);print(' '.join(f'{b:02X}' for b in prg[o:o+4]))"
# → 05 01 0A 01

# 2) CheckForItem 前 24 字节（07:EE4A）：ac 86 68 = ldy $6886
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=7*0x4000+(0xEE4A-0xC000);print(' '.join(f'{b:02X}' for b in prg[o:o+24]))"
# → AC 86 68 F0 12 A5 07 D9 86 68 D0 07 A5 06 D9 85 68 F0 05 88 88 D0 EE 18

# 3) MakeBitMask（07:DB2F）：38 A9 00 2A 88 10 FC 60
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=7*0x4000+(0xDB2F-0xC000);print(' '.join(f'{b:02X}' for b in prg[o:o+8]))"
# → 38 A9 00 2A 88 10 FC 60

# 4) 历史写入点只三处（普通道具 / 母脑 / Zebetite）
grep -rn "AddItemToHistory" refs/m1disasm/SRC/prg7_engine.asm
# → 5764 AddItemToHistory:（定义）/ 12796 jsr AddItemToHistory / 12826 jmp AddItemToHistory

# 5) 波束的 BG 碰撞后仍移动（07:D580）
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=7*0x4000+(0xD580-0xC000);print(' '.join(f'{b:02X}' for b in prg[o:o+12]))"
# → 20 09 D6 B0 03 20 24 D6 4C F8 D4 8F
#   jsr $D609(UpdateBullet_CollisionWithBG) / bcs +3 / jsr $D624(UpdateBullet_Move) / jmp $D4F8(CheckBulletStat)

# 6) Brinstar 特殊物品表首节点（01:A3D6）
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=1*0x4000+(0xA3D6-0x8000);print(' '.join(f'{b:02X}' for b in prg[o:o+16]))"
# → 02 E4 A3 03 05 04 03 00 0F FF 02 05 37 00 03 F3
```

## §6 C 级假设（不作结论）

- **H1**：`it_Mellow`（`$03`）被击破时是否也进 `UniqueItemHistory`——`AddItemToHistory` 全仓只有三处调用（普通道具 `:5764`、母脑 `:12796`、Zebetite `:12826`），Mellow 不走历史；但它是「特殊物品表里的可拾取条目」，实际生成路径在 `SpawnMellow`（不入历史），与密码系统的 `ItemData` 计数关系未核。
- **H2**：`SpawnMotherBrain` 的 `CheckForItem` 只看历史，母脑被击杀且未走到逃逸计时（例如 NARPASSWORD 跳过）时不会记历史 ⇒ 重进房间会重新生成。原版是否暴露该行为未实测（属 B7 进度链的已知边界）。
- **H3**：`ui_*` 类型低位为 0，`CheckForItem` 两字节全比；若同类型多件（5 座 Zebetite 各有独立类型码）不会冲突，但 `ui_ZEBETITE1+n` 的 `+n` 由 `txa/lsr`（槽索引×4）得到，槽序与房间绘制顺序的对应关系未逐帧核。

## §7 未覆盖

1. `DisplayHUD` 的完整像素布局与逃逸计时分支（本次只给字段级，未给 §4 每格坐标）。
2. 波束的**逐帧 tile 行为**——已核到调用点（`07:D580` 调 `UpdateBullet_CollisionWithBG`、撞后仍 `Move`）与 `IsBlastTile` 分支，但未逐帧核「哪类瓦破砖/不破砖」的完整集合。
3. 导弹门的「弹药扣减」与 `MissileCount` 交互（在发射链里，未核）。
4. `it_Mellow`/Squeept 生成的历史写入路径（见 H1）。
5. HUD 与密码系统的 `PasswordByte` 位图对应（属 B8 续作，另开卡）。
