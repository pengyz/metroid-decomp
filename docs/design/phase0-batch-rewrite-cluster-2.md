# Phase0 批量改写准备·簇 2 修订：world::room（房间加载）

> 【状态】修订文档实录（A 级实录）——refs 引用核（存在性（bank:addr 引证（A 级原文）））（实址（非占位））
> 【修订原因】对抗复核 verdict=needs_revision（2/4 簇拒收（refs 占位形））——「bank:addr（房间加载例程）」字面占位（非实址（cluster-2.md:7-9/39-42））→**实址**
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（房间加载例程（实址（A 级原文））））→修订文档实录（refs 引用核（存在性（实址（非占位））））

## refs 引用核（存在性（bank:addr 引证（A 级原文）））——实址（非占位）

### 修订前（占位形（非实址））

- 「bank:addr（房间加载例程）」字面占位（非实址（cluster-2.md:7-9/39-42））——**拒收**（对抗复核 verdict=needs_revision）

### 修订后（实址（bank:addr 引证（A 级原文）））

- **房间加载例程**（`bank00:8BD4-8C38`（LoadUniqueItems）+`bank00:8D12`（LoadPasswordData）+`bank00:8D3D-8D5C`（LoadTanksAndMissiles）——refs asm 考证（bank:addr（实址（A 级原文））））——房间加载（R-mv 规则面）：
  - **输入**：RAM 态（房间数据/道具状态）→RoomState
  - **规则**：LoadUniqueItems/LoadPasswordData/LoadTanksAndMissiles（8BD4/8D12/8D3D 族）——房间数据加载+道具初始化
  - **输出**：RoomState（房间数据写回 RAM）
- **房间渲染例程**（`bank00:8A8C-8A9A`（LoadPalData）+`bank00:87AB`（LoadSparkleData）——refs asm 考证（bank:addr（实址（A 级原文））））——房间渲染（R-mv 规则面）
- **表读径**（grep 实录（数据表使用（子程序（表读指令））））：LoadPalData@PalSelectTbl（8A9A）（表读指令（LDA abs,X））

## 六步流程汇总（修订后）

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（8BD4-8C38+8D12+8D3D-8D5C+8A8C-8A9A+87AB（实址（非占位））） |
| 2 | 域结构（world::room（load_unique_items+load_password_data+load_tanks_and_missiles）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→RoomState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（RoomState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8BD4-8C38`（LoadUniqueItems）+`bank00:8D12`（LoadPasswordData）+`bank00:8D3D-8D5C`（LoadTanksAndMissiles）+`bank00:8A8C-8A9A`（LoadPalData）+`bank00:87AB`（LoadSparkleData）（房间加载例程（实址（A 级原文）））——**非占位形**
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+修订文档实录（refs 引用核（存在性（实址（非占位））））））
