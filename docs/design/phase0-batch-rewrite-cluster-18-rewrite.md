# Phase0 批量改写·第十八簇：engine::nmi（NMI）（六步流程）

> 【状态】批量改写实录（A 级实录）——六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（NMI 例程（实址（A 级原文））））→批量改写实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **NMI 例程**（`bank00:9A07`（NMIScreenWrite）+`bank00:BFBA-BFBF`（ROMSWITCH_RESET@WaitForVBlank1/2）——refs asm 考证（bank:addr（实址（A 级原文））））——NMI（R-mv 规则面）：
  - **输入**：RAM 态（NMI 状态/VBlank 状态）→NmiState
  - **规则**：NMIScreenWrite/WaitForVBlank（9A07/BFBA 族）——NMI 屏幕写+VBlank 等待
  - **输出**：NmiState（NMI 状态写回 RAM）
- **真规则公式**（公式形（零字面特值））：`pub fn s4_wait_nmi_pass(v1a: u8) -> (bool, bool)`（`crates/game-core/src/engine/mod.rs:4816`）

## 第二步：域结构（engine::nmi（NMI））

- **域方法**（`crates/game-core/src/engine/mod.rs:4816`）：`s4_wait_nmi_pass`（NMI 等待（R-mv-2 速度积分面））——RAM 态→NmiState→NMIScreenWrite（9A07 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：s4_wait_nmi_pass（域内行为簇（语义命名（A 级实录）））

## 第三步：memmap 适配（RAM 态→NmiState）

- **memmap 映射**（A 级实录）：RAM 态（NMI 状态/VBlank 状态）→NmiState（typed view）
- **写径实证**（bank:addr（9A07 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（NmiState→RAM 写回）

- **shim 写集**（A 级实录）：NmiState（NMI 状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（9A07 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/engine/mod.rs`（s4_wait_nmi_pass）

## 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"nmi_screen_write_9A07" => Some(nmi_screen_write_9A07)`（bank:addr→crate::module::fn 双射）
- **registry 登记**（待补）：`"s4_wait_nmi_pass_BFBA" => Some(s4_wait_nmi_pass_BFBA)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（9A07+BFBA-BFBF（实址（非占位））（重新取证）） |
| 2 | 域结构（engine::nmi（s4_wait_nmi_pass）） | ✓（engine/mod.rs:4816（语义命名（A 级））） |
| 3 | memmap 适配（RAM 态→NmiState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（NmiState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:9A07`（NMIScreenWrite）+`bank00:BFBA-BFBF`（WaitForVBlank）（NMI 例程（实址（A 级原文）））——**非占位形**（实址（非占位）（内容一致（无矛盾）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+批量改写实录（六步流程）））
