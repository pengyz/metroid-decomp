# findings：L3 移动场景 B 首分歧分类报告（samus-move 900f G4 切片）

> 2026-10-16；task-28 会师首跑（decomp 条目 84a8a8a 放行后）。判据=首分歧可分类
> （boot-title 迭代分类模式）。

## 首跑裁定面

| 面 | 结果 |
|---|---|
| manifest 条目 | 885 帧/缺帧 [2,4,5,6,361-363,542-548,900]（decomp 如实登记）/head_skip=1 |
| game-core 腿 | **exit 101**——「unported @ bank07:B31B」 |
| 首分歧 | **B31B 声引擎分派**（RunSFXNoiseInitRoutine 族——nmikstas sound_engine.asm） |

## 首分歧分类（第 1 类）

### 类 m1：声引擎分派缺口（**任务卡·port2 task-30 分派层**）

- **现象**：tick 内 NMI 侧 sound_engine 分派触发 `unported!("bank07:B31B")`
  （SFXNoiseInitFlags≠0 面——移动场景含音效旗写入）。
- **分类**：声引擎域缺口（B31B 族四 init 子件（B31B/B34B/B33D/BC36）——task-30
  audio 独立域（链规 15）；lead 裁定 port2 分派层=会师前沿缺口 ✓ 预言字面命中）。
- **任务卡**：task-30 分派层清偿（port2 在途）——清偿后 B 跑续（预期次分歧=Samus
  移动深路径面（10 态肉的嵌套件族（F1FA/D147/CD6D 等））。
- **证据**：/tmp/porter/moveB.out 原文（exit 101+panic @ nmi.rs:184）。

## 预期后续分歧面（迭代备忘）

- **m2**：Samus 移动深路径嵌套件（D976 冻敌环几何族/CD6D SetSamusData/CF5D 链尾件
  ——10 态肉桩形面）。
- **m3**：门/电梯深路径（D3A8/D423 随件形）。
- **m4**：PPU 渲染面盲区（注记照留）。

## 复现命令

```
cargo run -q -p game-oracle --bin l3-dump -- --scenario samus-move
# 预期：exit 101 / unported @ bank07:B31B（task-30 清偿前）
```
