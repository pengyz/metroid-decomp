# Phase0 试点簇·第一簇：samus::physics::integrate_y（六步流程）

> 【状态】试点簇实录（A 级实录）——已有证明级等价（`integrators::int_y` 先例）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（int_y 例程（写径实证（真规则（公式形）））））→试点簇实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（写径实证（A 级原文））））

- **int_y 例程**（`bank07:E3A5-E3E4`——refs asm 考证（bank:addr（写径实证（A 级原文））））——垂直积分切片（R-mv 规则面）：
  - **输入**：RAM 三态（垂直速度/加速度/位置）→IntYState
  - **规则**：int_y（E3A5 族）——速度积分+正负 clamp（+5/-6 reset）
  - **输出**：IntYOut（速度/位置写回 RAM）
- **证明级等价**（`integrators::int_y` 先例）：**2^32 输入域穷举等价证明**（`crates/game-oracle/src/bin/int-y-proof.rs`——EXHAUSTIVE PROOF: int_y ≡ bank07:E3A5-E3E4（字节驱动语义）——0 分歧）
- **真规则公式**（公式形（零字面特值））：`pub fn int_y(st: &mut IntYState, accel: u8) -> IntYOut`（`crates/game-core/src/integrators.rs:35`）

## 第二步：域结构（samus::physics::integrate_y）

- **域方法**（`crates/game-core/src/samus/physics.rs:338`）：`integrate_y`（垂直积分拼装（R-mv-2 速度积分面））——RAM 三态→IntYState→int_y（E3A5 族）→写回
- **拼装位**（`crates/game-core/src/samus/physics.rs:325`）：积分器收编（task-25——int_x/int_y 证明级件拼装；R-mv 规则面）

## 第三步：memmap 适配（RAM 三态→IntYState）

- **memmap 映射**（A 级实录）：RAM 三态（垂直速度/加速度/位置）→IntYState（typed view）
- **写径实证**（bank:addr（E3A5 族（写径实证（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（IntYOut→RAM 写回）

- **shim 写集**（A 级实录）：IntYOut（速度/位置）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（E3A5 族（写径实证（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/integrators.rs`（int_y_plain_accumulation/int_y_positive_clamp_resets_to_5/int_y_negative_clamp_resets_to_minus6）

## 第六步：registry 登记+独立复审

- **registry 登记**（`crates/game-core/src/l2ports.rs:740`）：`"int_y_E3A5" => Some(int_y_e3a5)`（L2 ports registry）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（写径实证（A 级原文））） | ✓（E3A5-E3E4（写径实证）） |
| 2 | 域结构（samus::physics::integrate_y） | ✓（physics.rs:338（拼装位）） |
| 3 | memmap 适配（RAM 三态→IntYState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（IntYOut→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry ✓（l2ports.rs:740）——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:E3A5-E3E4`（int_y 例程（写径实证（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+试点簇实录（六步流程）））
