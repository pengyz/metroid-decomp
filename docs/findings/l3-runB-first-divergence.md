# findings：L3 首跑 B 首分歧分类报告（boot-title 300 帧）

> 2026-10-16；lead B 跑判据=「首分歧点可分类，每类一张任务卡」。本报告=两轮迭代
> 分类实录（首分歧→分类→修复/派卡→次分歧→再分类）。

## 裁定面总览

| 面 | oracle 腿 | game-core 腿 | 判定 |
|---|---|---|---|
| 恒 1 断言（$001A spin 出口） | 295 帧全 01 | **297 帧全 01**（修复后） | ✅ 双腿过（decomp task-24 迁移面闭环） |
| 300 帧全跑 | ✅ | ✅（dump 落盘 exit 0） | ✅ B 跑形态成立 |
| 首分歧 | 帧集合 | 帧集合 | 见类 b2 |

## 分类实录（每类一张任务卡）

### 类 b1：采样点相位对齐（**已修**）

- **现象**（run B1）：首分歧 帧 1 字段 zp CPU $001A=0x00（≠01）——NMI 面退化信号。
- **分类**：采样对齐（boot tick（PreSpin/AwaitNmi/PostSpin）无主循环 spin，不应采样
  ——oracle 295 行=主循环 spin 出口数自证（300-5 boot））。
- **处置**：缝语义限 **MainLoop 相**（tick_frame_with 只在主循环 spin 出口采样）——
  修复后恒 1 双腿全过。
- **证据**：修复前后 diff 输出对照（runB_diff.out:「frame 1 … $001A=0x00」→
  runB2_diff.out:「恒 1 断言过（game-core）：297 帧 $001A 全 01」）。

### 类 b2：帧编号集合对齐（**任务卡·decomp 侧 l3-dump f 键面**）

- **现象**（run B2）：「首分歧 帧集合不一致（左缺=[4, 5, 6, 300] 右缺=[1, 3]）」
  ——oracle 帧号缺 4/5/6/300（Mesen 场景帧计数在 boot 转段跳号），game-core 缺 1/3
  （tick 编号 vs 场景帧号）。
- **分类**：帧编号键面（f 键=场景帧号 vs tick 序号——task-24 采样迁移的对表残项）。
- **任务卡**：l3-dump 的 f 键改用场景帧号（oracle manifest 对表——decomp 侧）；
  或 diff_traces 帧集合对齐白名单（若为采样口径差）。
- **证据**：runB2_diff.out 原文。

### 预期面备忘（未及——帧号对齐后下一跑）

- **c) 5 件 L3 降级例程深路径**（01/12/15/1B/1C——占位臂记账 0x80D0 等）：占位臂
  NmiYield 让出+首分歧记账（DIVERGENCES 面已落）——深效果缺失=预期 L3 裁决面。
- **d) PPU 渲染面盲区**（CHR 写不可观测——watch 面外注记照留）。

## 复现命令

```
cargo run -q -p game-oracle --bin l3-dump -- --scenario boot-title
python3 tools/diff_traces.py --manifest analysis/dump/l3_manifest.json
# 预期：恒 1 双腿过 + 首分歧=帧集合（类 b2）
```
