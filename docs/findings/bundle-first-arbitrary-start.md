# findings：首例任意起点对拍（task-32 收官件——bundle 全链实战）

> 状态：**全链机制落成**（2026-10-16）；双腿对拍的游戏腿推进=移动链移植前沿
> （task-30/B31B+C23x），机制面无阻。
> 复跑：`scenario_run run bundle-capture` → `scenario_run run bundle-ff` →
> `cargo run -p game-oracle --bin l3-dump -- --scenario bundle-ff --bundle
> analysis/bundles/mid-move`。

## 1. 全链四步（输出原文）

| 步 | 实证 |
|---|---|
| ① bundle 采集 | `bundle captured f=900（目标 900，首个 ≥ 采样点）mss=11915B` + 三直映 JSON（ram/wram/regs）——**exec 回调上下文采集**（约束满足） |
| ② oracle 恢复 | `bundle load f=1 bytes=11915 ok=true`——MSS 全机恢复 |
| ③ 快进 240f | bundle-ff 237 行（右持窗） |
| ④ 对拍 | **102 帧逐字节忠实**（ff 行 i ↔ 连续 f900+i）；首分歧 ff f=107↔cont f=1004 zp$08（94/91） |

- **任意起点 102 帧忠实重放** ✓（装载态与连续态逐字节同）；
- 首分歧分类：**恢复相位面**（~100 帧后漂移——PPU 帧界/输入相位在装载时刻的对齐
  残差，savestate 重放经典类）——残差显式，非机制失败；
- 映射偏移口径：ff 行 k ↔ 连续 F_a+k（F_a=采集帧 900；首行=装载后首 spin 出口）。

## 2. 双腿联合验收（porter 映射件 a72d37a）

- **game-core 恢复腿**：`restore_mss(ram, wram, regs)` 生效（l3-dump --bundle 通道
  =本件交付）——恢复态起跑至移动链深路 **`unported @ bank07:C23x-change`**（nmi.rs:221）
  引爆=fail-loud 不落盘 ✓；
- 判定：**映射件功能面证成**（恢复态连贯推进至 NMI 路径移植前沿）；双腿逐帧对拍
  待 task-30（B31B/C23x 移动链）落——机制无阻。

## 3. 余件：diff_traces 三条件断言消费（实证过）

```
[note] l3-fixture-three-conditions: 三条件断言过（2 fixture：(b) 读回+(c) 持久）
```

- fixture 声明（case.fixtures）→ (b) F_a+1 读回断言 + (c) invariant 窗口末持久断言
  自动消费；(a)=compare_frames 声明域全等本体；fixture-test 案（fixture 200 $0700/$0701）
  实证过（与 spec §3 实战补账同源数据）。

## 4. 工程修正（实战坑三件）

1. **bundle 触发帧语义**：严格相等 f==F_a 错过缺帧（f=900 spin 出口空窗实录）→
   「首个 ≥F_a 采样点」稳健语义；
2. **相对路径崩回调**：io.open 相对路径在 Mesen CWD 下静默失败→nil 写崩采样回调
   （行止 899 严丝合缝定位）→ scenario_run 解析绝对路径+写盘 fail-loud 包装；
3. fixture 写面 MEMTYPE=8（探针口径迁移）。

## 5. 遗留

1. 双腿逐帧对拍待 task-30 移动链（B31B/C23x）落——bundle-ff 案已就位（manifest
   legs.game-core 注）；
2. 恢复相位残差（~100 帧漂移）根因细化（PPU 帧界对齐/输入相位）——bundle v2 面
   （S6 PPU 面补全后复测）；
3. 三条件 (a) 的双腿形态（game-core 腿 bundle 恢复后 state-eq）随 1 落。
