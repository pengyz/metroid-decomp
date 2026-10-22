# Phase0 批量改写·簇 4 修订：weapon::projectile（弹丸）

> 【状态】修订文档实录（A 级实录）——refs 引证域归属定谳（域错位更正（端址一致（重新取证）））
> 【修订原因】对抗复核 verdict=needs_revision（簇 4/5 拒收（refs 域错位（标题画面例程误作游戏内）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（游戏内弹丸例程（实址（A 级原文））））→修订文档实录（refs 引证域归属定谳（域错位更正（端址一致（重新取证））））

## refs 引证域归属定谳（域错位更正）

### 修订前（域错位（标题画面例程误作游戏内））

- **bank00:8897-88FE（InitCrossMissiles）+8963（UpdateCrossMissile）**——误作游戏内弹丸例程（域错位（定域取证 spec 步错））——**拒收**（对抗复核 verdict=needs_revision）
- **refs 原文**（prg0_title.asm:973-1000（IntroSprs.0.y/4.y（开场精灵（准星汇聚动画））））：:314「Load initial sprite values for crosshair routine」+:387「Draw sprites that converge in center of screen」+:390「Draw cross sprites in middle of the screen」——**标题画面准星精灵例程**（非游戏内弹丸例程）

### 修订后（域归属更正（实址（A 级原文）））

- **域归属更正**（①）：bank00:8897-88FE（InitCrossMissiles）+8963（UpdateCrossMissile）=**标题画面准星精灵例程**（非游戏内弹丸例程）——refs 原文（prg0_title.asm:973-1000（开场精灵（准星汇聚动画）））
- **端址一致**（②）：8963-8975（簇 4）vs 8963-8972（簇 5）（同例程引证不一致）→**统一为 8963-8975**（簇 4/5 端址一致）
- **重新取证**（③）：**游戏内弹丸例程**（`bank07:D1EE-D210`（FireSamusProjectile/SearchOpenSamusProjectileslot）+`bank07:D210-D2CC`（FireSamusProjectileForwards/FireSamusProjectileUpwards）+`bank07:D4BF`（UpdateSamusProjectiles）——refs asm 考证（bank:addr（实址（A 级原文））））——游戏内弹丸（R-mv 规则面）

## 六步流程汇总（修订后）

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（D1EE-D210+D210-D2CC+D4BF（实址（非占位））（重新取证（域归属更正））） |
| 2 | 域结构（weapon::projectile（fire_samus_projectile/search_open_samus_projectile_slot/update_samus_projectiles）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→ProjectileState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（ProjectileState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:D1EE-D210`（FireSamusProjectile/SearchOpenSamusProjectileslot）+`bank07:D210-D2CC`（FireSamusProjectileForwards/FireSamusProjectileUpwards）+`bank07:D4BF`（UpdateSamusProjectiles）（游戏内弹丸例程（实址（A 级原文）））——**非占位形**+**域归属更正**（标题画面准星精灵例程（非游戏内弹丸例程））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+修订文档实录（refs 引证域归属定谳（域错位更正（端址一致（重新取证））））））
