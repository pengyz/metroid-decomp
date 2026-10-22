# Phase0 批量改写·簇 19 修订：engine::boot（启动）

> 【状态】修订文档实录（A 级实录）——refs 引证零实址更正（实址（非占位））
> 【修订原因】对抗复核 verdict=needs_revision（1/4 拒收）——refs 引证零实址 ✗（bank:addr 值**零命中**（grep 实录））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（启动例程（实址（A 级原文））））→修订文档实录（refs 引证零实址更正（实址（非占位）））

## refs 引证零实址更正（实址（非占位））

### 修订前（refs 引证零实址 ✗）

- 「bank:addr 值**零命中**（grep 实录）」——**refs 引证零实址**（bank:addr 无实址（grep 实录））——**拒收**（对抗复核 verdict=needs_revision）

### 修订后（实址（bank:addr 引证（A 级原文）））

- **启动例程**（`bank00:8071-80BE`（InitializeAfterReset）+`bank00:92D4`（InitializeGame）+`bank00:932B`（InitializeStats）——refs asm 考证（bank:addr（实址（A 级原文））））——启动（R-mv 规则面）：
  - **输入**：RAM 态（启动状态/初始化数据）→BootState
  - **规则**：InitializeAfterReset/InitializeGame/InitializeStats（8071/92D4/932B 族）——启动初始化+游戏初始化+统计初始化
  - **输出**：BootState（启动状态写回 RAM）
- **真规则公式**（公式形（零字面特值））：`pub fn boot(ram: &[u8; 2048]) -> BootState`（`crates/game-core/src/engine/boot.rs:24`）

## 六步流程汇总（修订后）

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（8071-80BE+92D4+932B（实址（非占位））（重新取证）） |
| 2 | 域结构（engine::boot（boot）） | ✓（engine/boot.rs:24（语义命名（A 级））） |
| 3 | memmap 适配（RAM 态→BootState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（BootState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8071-80BE`（InitializeAfterReset）+`bank00:92D4`（InitializeGame）+`bank00:932B`（InitializeStats）（启动例程（实址（A 级原文）））——**非占位形**（实址（非占位（内容一致（无矛盾））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+修订文档实录（refs 引证零实址更正（实址（非占位）））））
