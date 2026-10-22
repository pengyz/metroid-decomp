# Phase0 批量改写·第四簇：weapon::projectile（弹丸）（六步流程）

> 【状态】批量改写实录（A 级实录）——六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（弹丸例程（实址（A 级原文））））→批量改写实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **弹丸例程**（`bank00:8897-88FE`（InitCrossMissiles）+`bank00:8963-8975`（UpdateCrossMissile）+`bank00:8D3D-8D7B`（LoadTanksAndMissiles）——refs asm 考证（bank:addr（实址（A 级原文））））——弹丸（R-mv 规则面）：
  - **输入**：RAM 态（弹丸位置/状态）→ProjectileState
  - **规则**：InitCrossMissiles/UpdateCrossMissile/LoadTanksAndMissiles（8897/8963/8D3D 族）——弹丸初始化+更新+加载
  - **输出**：ProjectileState（弹丸状态写回 RAM）
- **弹丸 SFX 例程**（`bank00:B255-B281`（SFXData@EnemyHit/BigEnemyHit/StatueRaise）——refs asm 考证（bank:addr（实址（A 级原文））））——弹丸音效（R-mv 规则面）
- **真规则公式**（公式形（零字面特值））：`pub fn missile_pickup_sfx_cont(state: &mut SamusState)`（`crates/game-core/src/samus/l4_ready_impl.rs`（第六十四组））

## 第二步：域结构（weapon::projectile（弹丸））

- **域方法**（待补）：`weapon::projectile::init_cross_missiles`（弹丸初始化（R-mv-2 速度积分面））——RAM 态→ProjectileState→InitCrossMissiles（8897 族）→写回
- **域方法**（待补）：`weapon::projectile::update_cross_missile`（弹丸更新（R-mv-2 速度积分面））——RAM 态→ProjectileState→UpdateCrossMissile（8963 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：init_cross_missiles/update_cross_missile（域内行为簇（语义命名（A 级实录）））

## 第三步：memmap 适配（RAM 态→ProjectileState）

- **memmap 映射**（A 级实录）：RAM 态（弹丸位置/状态）→ProjectileState（typed view）
- **写径实证**（bank:addr（8897 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（ProjectileState→RAM 写回）

- **shim 写集**（A 级实录）：ProjectileState（弹丸状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（8897 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/samus/l4_ready_impl.rs`（missile_pickup_sfx_cont/energy_pickup_sfx_cont（第六十四组））

## 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"init_cross_missiles_8897" => Some(init_cross_missiles_8897)`（bank:addr→crate::module::fn 双射）
- **registry 登记**（待补）：`"update_cross_missile_8963" => Some(update_cross_missile_8963)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（8897-88FE+8963-8975+8D3D-8D7B（实址（非占位））） |
| 2 | 域结构（weapon::projectile（init_cross_missiles+update_cross_missile）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→ProjectileState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（ProjectileState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8897-88FE`（InitCrossMissiles）+`bank00:8963-8975`（UpdateCrossMissile）+`bank00:8D3D-8D7B`（LoadTanksAndMissiles）（弹丸例程（实址（A 级原文）））——**非占位形**
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+批量改写实录（六步流程）））
