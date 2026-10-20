# findings：G4 通关集成卡包预备（Tourian→MotherBrain→escape→ChooseEnding）

> 状态：**G4 证据预备仓就位**（2026-10-20）。a 系形（同 g3/arm 包）。
> refs asm 先读纪律照执行。首件随至（卡包骨架+簇清单勘）。

## 1. G4 簇清单（refs asm 先读——通关集成域）

| 簇 | 员 | 成员（addr+名） |
|---|---|---|
| G4-00 结局 | 1 | ChooseEnding($CAF5) |
| G4-01 音乐 | 2 | TourianMusic($CC07)/MotherBrainMusic($CC03) |
| G4-02 boss | 1 | SpawnMotherBrain($EEAE) |
| G4-03 逃生 | 3 | EndTimer($010A dw——逃生计时器)/MotherBrainTimeBombCounter($99)/music_Escape（%00000100）|

## 1.5 ChooseEnding 推导深（A 级：prg7:CAF5-CB09）

- **ChooseEnding 全形**（$CAF5）：y=1..4 循环——SamusAge+2 判（≠0=最差结局——>37h）
  +AgeTable-1,y 比较（cmp bcs——时间阈表）——**5 结局选择**（y 序=好→差）；
- **⚠ disasm 内联 BUG 注**（CAF9 处）：「BUG! Should be checking SamusAge+3 here too」
  ——**SamusAge+3 漏检缺陷**（D4 候选注——保真保留）；
- **写集**：结局选择态（AgeTable 查表）；
- **G4 接面注**：ChooseEnding（结局）+TourianMusic/MotherBrainMusic（通关音乐）+
  SpawnMotherBrain（boss 生成）——通关集成四面。

## 2. 行数核自查条

- 本包=簇清单全（G4-00..03）——自查：4 簇/7 员 ✓。

## 3. 【2026-10-20 通关径场景包预备】（G2 消费方形——G4 接面）

- **通关径场景骨架**（G2 消费方形）：Tourian→MotherBrain→escape→ChooseEnding 四段
  ——**场景包待采**（通关径实跑=远距路由（G2 触达战役教训：22 迭代收））；
- **G4 消费方接面**：ChooseEnding（结局判——AgeTable）+MotherBrain（boss 战）+
  逃生（EndTimer）——**素材→场景环预备**（G2 形照用）；
- **收束报协同注**：G4 素材面（簇清单+ChooseEnding+逃生族）全落——场景包=通关径
  实跑件（远距路由设计=后续大件）。

## 4. 【2026-10-20 G5 收官协同】分母账面终形（D7 面）

- **分母账面终形**（completeness_denoms v2）：**S=492/D=193/B=89/R=0**（covered 57）
  ——v2 纠偏：+34 S 格/错面待迁 27/段切 1/残余 0（显式）；
- **导出态**：+78 新格/654 保留——分母只登记 L0（分子由角色任务带证据升级）；
- **D7 面核**：S/D 机械分母 ✓（B 行为空间=声明性覆盖/R 不可证=S4 起登记）——
  **分母账面终形稳**。

### 4.1 Gate 面协同（2026-10-20）

- **全闸态**：15 PASS/0 FAIL/0 SKIP——唯 WARN=rust_test 532（基线 365——批量上调候裁）；
- **Gate 面核**：rom-integrity/seed/命名台账 182/知识 34/docs-links 174/rust 532/
  L2 全绿/L3 22 案/L4 50 金面/completeness 57/tree-coherence/doc-sync/inode 锁 0
  ——**Gate 面稳**（14 闸全绿+ratchet 七向 OK）。

### 4.2 L4 3 项账清转 strict 路径（2026-10-20 收敛族协同）

- **L4 3 项账内容忍**（sc-title-menu2_f00620 等——迭代态缺口 12759 像素族）：
  TOLERATED 注「账清转 strict」——**转 strict 路径**：①迭代态缺口收敛（渲染面批产
  完成）②白名单清（账内容忍→strict 裁定）③重跑 visual-diff 确认 0 像素差；
- **收敛族协同注**：L4 3 项+WARN 1 项（rust_test 批量上调候裁）——**双收敛路径
  显式**（账清转 strict/基线上调）。

## 5. 【2026-10-20 L2 收敛最高优先】建模清类协同（清零支撑）

- **建模清类候选**（缺口路线图在列——VALCHAIN/ACHAIN/XCHAIN/AYCHAIN/AMUL16/
  ADIV16 族）：**86 条**（值链算/链算径/乘除径）——**L2 收敛清零支撑态**；
- **账外 0**（L2 账面制全账内）；**TOLERATED 消费协同**（L4 3 项账内容忍在列）；
- **使命序**（用户裁定）：L2 收敛=最高优先→对拍场景矩阵设计位（L2 清后）。
