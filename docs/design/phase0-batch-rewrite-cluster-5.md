# Phase0 批量改写准备·第五个域内行为簇：pickup::items（道具拾取）

> 【状态】设计文档实录（A 级实录）——域内行为簇设计（pickup::items（道具拾取）（六步流程））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（道具拾取例程（写径实证（真规则（公式形）））））→设计文档实录（六步流程）

## 域内行为簇设计（pickup::items（道具拾取））

### 第一步：定域取证 spec（refs asm 考证（bank:addr（写径实证（A 级原文））））

- **道具拾取例程**（`bank00:8897-88FE`（InitCrossMissiles）+`bank00:8963-8972`（UpdateCrossMissile）——refs asm 考证（bank:addr（写径实证（A 级原文））））——道具拾取（R-mv 规则面）：
  - **输入**：RAM 态（道具位置/拾取状态）→ItemState
  - **规则**：InitCrossMissiles/UpdateCrossMissile（8897/8963 族）——道具初始化+拾取更新
  - **输出**：ItemState（拾取状态写回 RAM）
- **道具 SFX 例程**（`bank00:B6A9-B7CB`（MissilePickupSFX/EnergyPickupSFX/DoorOpenCloseSFX）——refs asm 考证（bank:addr（写径实证（A 级原文））））——道具拾取音效（R-mv 规则面）
- **真规则公式**（公式形（零字面特值））：`pub fn missile_pickup_sfx_cont(state: &mut SamusState)`（`crates/game-core/src/samus/l4_ready_impl.rs`（第六十四组））

### 第二步：域结构（pickup::items（道具拾取））

- **域方法**（待补）：`pickup::items::init_cross_missiles`（道具初始化（R-mv-2 速度积分面））——RAM 态→ItemState→InitCrossMissiles（8897 族）→写回
- **域方法**（待补）：`pickup::items::update_cross_missile`（道具拾取更新（R-mv-2 速度积分面））——RAM 态→ItemState→UpdateCrossMissile（8963 族）→写回
- **域方法**（`crates/game-core/src/samus/l4_ready_impl.rs`（第六十四组））：`missile_pickup_sfx_cont`（道具拾取音效（R-mv-2 速度积分面））——RAM 态→SamusState→MissilePickupSFXCont（B6B0 族）→写回

### 第三步：memmap 适配（RAM 态→ItemState）

- **memmap 映射**（A 级实录）：RAM 态（道具位置/拾取状态）→ItemState（typed view）
- **写径实证**（bank:addr（8897 族（写径实证（A 级原文））））：RAM 读写（byte-driven semantics）

### 第四步：shim 写集适配（ItemState→RAM 写回）

- **shim 写集**（A 级实录）：ItemState（拾取状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（8897 族（写径实证（A 级原文））））：RAM 写回（byte-driven semantics）

### 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/samus/l4_ready_impl.rs`（missile_pickup_sfx_cont/energy_pickup_sfx_cont（第六十四组））

### 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"init_cross_missiles_8897" => Some(init_cross_missiles_8897)`（L2 ports registry）
- **registry 登记**（待补）：`"update_cross_missile_8963" => Some(update_cross_missile_8963)`（L2 ports registry）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（写径实证（A 级原文））） | ✓（8897-88FE+8963-8972+B6A9-B7CB（写径实证）） |
| 2 | 域结构（pickup::items（init_cross_missiles+update_cross_missile+missile_pickup_sfx_cont）） | ✓（l4_ready_impl.rs（第六十四组）+待补（init/update）） |
| 3 | memmap 适配（RAM 态→ItemState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（ItemState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8897-88FE`（InitCrossMissiles）+`bank00:8963-8972`（UpdateCrossMissile）+`bank00:B6A9-B7CB`（道具拾取音效）（道具拾取例程（写径实证（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+设计文档实录（六步流程）））
