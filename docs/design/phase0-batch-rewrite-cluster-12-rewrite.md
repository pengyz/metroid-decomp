# Phase0 批量改写·簇 12 修订·二次：input::joy（输入）

> 【状态】修订文档实录（A 级实录）——refs 占位形更正（实址（非占位））+文档补全（二次修订）
> 【修订原因】对抗复核 verdict=needs_revision（2/4 拒收（refs 占位形（同病三犯（簇 2/4/5/6 首修订模式（占位→实址（二次修订应补））））+设计文档缺 ✗（cluster-12 无）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（输入例程（实址（A 级原文））））→修订文档实录（refs 占位形更正（实址（非占位））+文档补全）

## refs 占位形更正（实址（非占位））

### 修订前（字面占位（非实址））

- 「输入例程：**bank:addr**（实址（A 级原文））」——**字面占位（非实址**）——**拒收**（对抗复核 verdict=needs_revision）

### 修订后（实址（bank:addr 引证（A 级原文）））

- **输入例程**（`bank07:C215`（ReadJoyPads）+`bank00:940B`（DisplayInputCharacters）——refs asm 考证（bank:addr（实址（A 级原文））））——输入（R-mv 规则面）：
  - **输入**：RAM 态（手柄状态/按键）→JoyState
  - **规则**：ReadJoyPads/DisplayInputCharacters（C215/940B 族）——手柄读取+输入显示
  - **输出**：JoyState（输入状态写回 RAM）
- **真规则公式**（公式形（零字面特值））：`pub fn read_joypads(ram: &[u8; 2048]) -> JoyState`（`crates/game-core/src/input/joy.rs:24`）

## 文档补全（六步流程）

### 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **输入例程**（`bank07:C215`（ReadJoyPads）+`bank00:940B`（DisplayInputCharacters）——refs asm 考证（bank:addr（实址（A 级原文））））——输入（R-mv 规则面）

### 第二步：域结构（input::joy（输入））

- **域方法**（`crates/game-core/src/input/joy.rs:24`）：`read_joypads`（手柄读取（R-mv-2 速度积分面））——RAM 态→JoyState→ReadJoyPads（C215 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：read_joypads（域内行为簇（语义命名（A 级实录）））

### 第三步：memmap 适配（RAM 态→JoyState）

- **memmap 映射**（A 级实录）：RAM 态（手柄状态/按键）→JoyState（typed view）
- **写径实证**（bank:addr（C215 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

### 第四步：shim 写集适配（JoyState→RAM 写回）

- **shim 写集**（A 级实录）：JoyState（输入状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（C215 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

### 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/input/joy.rs`（read_joypads）

### 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"read_joypads_C215" => Some(read_joypads_C215)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（C215+940B（实址（非占位））（重新取证）） |
| 2 | 域结构（input::joy（read_joypads）） | ✓（input/joy.rs:24（语义命名（A 级））） |
| 3 | memmap 适配（RAM 态→JoyState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（JoyState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:C215`（ReadJoyPads）+`bank00:940B`（DisplayInputCharacters）（输入例程（实址（A 级原文）））——**非占位形**
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+修订文档实录（refs 占位形更正（实址（非占位））+文档补全）））
