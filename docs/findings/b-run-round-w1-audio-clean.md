# B 跑验证轮分类（W1 全清后）——首分歧=F1xx-flow 族（samus 移动臂域）

> 【触发】lead 调度裁定：W1 残面 4/4 全清后 B 验证轮先行（boot-title 300 帧+首分歧分类）。
> 【执行】`l3-dump --scenario boot-title` + `tools/diff_traces.py`（我落我验口径）。
> 【结论】二分命中 **②新分歧（账外）**——音频/标题面全清实证；首分歧=**帧 1 zp $0005**
> 归属 **F1xx-flow 族（samus 移动臂——porter 线）**，非 title/sound 面。

## 1. 验证结果（二分判读）

| 面 | 结果 |
|---|---|
| **缺口记账** | **0 事件/0 唯一址**（300 帧全跑——B 跑首次全跑通！） |
| 音频面（W1 全清） | **全绿**（无 unported 记账/无音频面分歧） |
| 标题面（批6b 全清） | **全绿** |
| 首分歧（账外） | 帧 1 **zp $0005**：game-core=0x40 vs oracle=0x00（`L3 帧级裁定: 14 案 1 失败`） |

**重大节点**：300 帧全跑+零记账=B 跑面（boot-title）**strict 绿的记账面达成**——
余分歧=账外行为级（分类体系的下一层）。

## 2. 首分歧分类（写手定位链）

| 证据 | 内容 |
|---|---|
| 帧/址 | 游戏帧 1 / zp($0000)+0x005 = CPU $0005 |
| 值 | 左(oracle)=00… / 右(game-core)=40…（单字节 0x00 vs 0x40） |
| 写手扫描 | 我域（title/sound）**无 $05 写面**；命中 `memmap/flow_scratch.rs:54 RAD_X=$05`+`l2ports.rs:275/301/355/371/388` 的 **(0x0004/0x0005) 速度/半径面写** |
| **归属** | **F1xx-flow 族 = samus 移动臂（porter 线）**——l2ports 共享注册面的 flow 适配族 |
| dump 佐证 | game-core 腿帧 4 态：$04=1F $05=40（flow 面写值）；$E0-$E3=00（音频指针面未写=该帧未触发 handler ✓ 正常） |

## 3. 缺口地图现状

| 行 | 状态 |
|---|---|
| title 例程行（80D0/82ED/…/9147/911A） | ✅ 全清 |
| bank07:B31B 声引擎分派行 | ✅ 全清（task-30 分派核+钩件） |
| handler 细行（B673/B650/…） | ✅ 已清 14 件（SamusHit/W2×6/W3×6） |
| W1 音频骨架（深径/VM/音符载/包络尾） | ✅ 全清 |
| **帧 1 zp $05（F1xx-flow 族语义）** | **新顶行**（账外行为级——samus 移动臂域） |

## 4. 复现命令

```bash
cargo run -p game-oracle --bin l3-dump -- --scenario boot-title
# → 300 帧 watch 向量 | 缺口记账 0 事件/0 唯一址
python3 tools/diff_traces.py
# → [FAIL] l3-boot-title-oracle-determinism: 首分歧 游戏帧 1 字段 zp @zp($0000)+0x005
```

【分类】A 级（执行实录+写手扫描链+dump byte 级对读）；【置信】91
