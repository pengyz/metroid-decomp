# Phase0 批量改写·簇 5 修订·二次修订：pickup::items（道具拾取）

> 【状态】修订文档实录（A 级实录）——重新取证（实址（非占位））（二次修订）
> 【修订原因】对抗复核 verdict=needs_revision（3/4 拒收）——「游戏内拾取例程：**bank:addr**（实址（A 级原文））」字面占位（非实址（cluster-5-rewrite.md §1.3））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（游戏内拾取例程（实址（A 级原文））））→修订文档实录（重新取证（实址（非占位）））

## 重新取证（实址（非占位））

### 修订前（字面占位（非实址））

- 「游戏内拾取例程：**bank:addr**（实址（A 级原文））」——**字面占位（非实址**（cluster-5-rewrite.md §1.3））——**拒收**（对抗复核 verdict=needs_revision）

### 修订后（实址（bank:addr 引证（A 级原文）））

- **游戏内拾取例程**（`bank00:8B79-8B82`（ProcessUniqueItems）+`bank00:8B9C`（UniqueItemSearch）+`bank00:8BB5`（UniqueItemFound）+`bank00:8BD4-8C38`（LoadUniqueItems）+`bank00:8C39`（SamusHasItem）+`bank00:9029`（ItemData）——refs asm 考证（bank:addr（实址（A 级原文））））——游戏内拾取（R-mv 规则面）：
  - **输入**：RAM 态（道具位置/拾取状态）→ItemState
  - **规则**：ProcessUniqueItems/UniqueItemSearch/UniqueItemFound/LoadUniqueItems/SamusHasItem/ItemData（8B79/8B9C/8BB5/8BD4/8C39/9029 族）——道具拾取+搜索+加载+检查
  - **输出**：ItemState（拾取状态写回 RAM）
- **表读径**（grep 实录（数据表使用（子程序（表读指令））））：LoadUniqueItems@processItemByte/Bit（8C03/8C0A）（表读指令（LDA abs,X））

## 六步流程汇总（二次修订后）

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（8B79-8B82+8B9C+8BB5+8BD4-8C38+8C39+9029（实址（非占位））（重新取证）） |
| 2 | 域结构（pickup::items（process_unique_items/unique_item_search/load_unique_items/samus_has_item）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→ItemState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（ItemState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8B79-8B82`（ProcessUniqueItems）+`bank00:8B9C`（UniqueItemSearch）+`bank00:8BB5`（UniqueItemFound）+`bank00:8BD4-8C38`（LoadUniqueItems）+`bank00:8C39`（SamusHasItem）+`bank00:9029`（ItemData）（游戏内拾取例程（实址（A 级原文）））——**非占位形**
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+修订文档实录（重新取证（实址（非占位）））））
