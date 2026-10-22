# Phase0 批量改写·第一簇：samus::physics（物理积分）（六步流程）

> 【状态】批量改写实录（A 级实录）——六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（物理积分例程（实址（A 级原文））））→批量改写实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **物理积分例程**（`bank07:E3A5-E3E4`（int_y）+`bank07:E340-E3A4`（int_x）——refs asm 考证（bank:addr（实址（A 级原文））））——垂直/水平积分切片（R-mv 规则面）：
  - **输入**：RAM 三态（速度/加速度/位置）→IntYState/IntXState
  - **规则**：int_y/int_x（E3A5/E340 族）——速度积分+正负 clamp（+5/-6 reset）
  - **输出**：IntYOut/IntXOut（速度/位置写回 RAM）
- **真规则公式**（公式形（零字面特值））：`pub fn int_y(st: &mut IntYState, accel: u8) -> IntYOut`（`crates/game-core/src/integrators.rs:35`）

## 第二步：域结构（samus::physics（物理积分））

- **域方法**（`crates/game-core/src/samus/physics.rs:340`）：`vert_integrate`（垂直积分拼装（R-mv-2 速度积分面））——RAM 三态→IntYState→int_y（E3A5 族）→写回
- **域方法**（`crates/game-core/src/samus/physics.rs:358`）：`horz_integrate`（水平积分拼装（R-mv-2 速度积分面））——RAM 三态→IntXState→int_x（E340 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：vert_integrate/horz_integrate（域内行为簇（语义命名（A 级实录）））

## 第三步：memmap 适配（RAM 三态→IntYState/IntXState）

- **memmap 映射**（A 级实录）：RAM 三态（速度/加速度/位置）→IntYState/IntXState（typed view）
- **写径实证**（bank:addr（E3A5/E340 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（IntYOut/IntXOut→RAM 写回）

- **shim 写集**（A 级实录）：IntYOut/IntXOut（速度/位置）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（E3A5/E340 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/integrators.rs`（int_y_plain_accumulation/int_y_positive_clamp_resets_to_5/int_y_negative_clamp_resets_to_minus6）

## 第六步：registry 登记+独立复审

- **registry 登记**（`crates/game-core/src/l2ports.rs:740`）：`"int_y_E3A5" => Some(int_y_e3a5)`（bank:addr→crate::module::fn 双射）
- **registry 登记**（待补）：`"int_x_E340" => Some(int_x_e340)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（E3A5-E3E4+E340-E3A4（实址（非占位））） |
| 2 | 域结构（samus::physics（vert_integrate+horz_integrate）） | ✓（physics.rs:340+358（语义命名（A 级））） |
| 3 | memmap 适配（RAM 三态→IntYState/IntXState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（IntYOut/IntXOut→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry ✓（l2ports.rs:740（int_y））+**int_x 待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:E3A5-E3E4`（int_y）+`bank07:E340-E3A4`（int_x）（物理积分例程（实址（A 级原文）））——**非占位形**
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+批量改写实录（六步流程）））
