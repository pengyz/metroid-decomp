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
