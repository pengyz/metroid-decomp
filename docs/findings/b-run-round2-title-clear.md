# B 跑次轮（title 账清验证轮）分类报告

> 【范围】里程碑 B boot-title 臂判据验证：批6b 清偿后（0x18/0x1E 落）title 面账清验证。
> 【执行】`cargo run -p game-oracle --bin l3-dump -- --scenario boot-title`（300 帧全跑意图）。
> 【结论】**title 面已执行前缀零记账**；首分歧=**bank07:B31B（声引擎分派）**——task-30
> 进行中件，账面预期演化（B31B 行→handler 细行）。

## 1. 首分歧分类

| 项 | 值 |
|---|---|
| 首分歧点 | `unported @ bank07:B31B`（frame/nmi.rs:184） |
| 分类 | **声引擎域件缺口**（task-30 在途——批6b 集成发现的新缺口行） |
| 触发链 | 01 DrawIntroBackground 段0 忠实写 `$0684←$10`（SFXMultiInitFlags）→ NMI 侧声分派 → B31B 未端口引爆 |
| title 面记账 | **零**（已执行前缀内 0x00/0x01 臂全武装——无 bank00: 表址记账） |

## 2. 与首轮对比（账面演化实证）

| 轮 | 首分歧 | 形态 |
|---|---|---|
| 首轮 | `bank00:80D0 ×297`（01 占位臂逐 tick 记账）+五降级件行 | 占位记账形（NmiYield 让出≠引爆）——300 帧跑通 |
| **次轮** | **`bank07:B31B` 单行**（声引擎） | title 臂忠实写触发声缺口——早段引爆（帧 ~3-4） |

**演化判读**：title 账面从「297×占位记账」→「零」（清偿达成）；缺口面从 title 例程行
→声引擎行（01 的 SFX 写为忠实行为（D4/Stage A 保真）——不是回归是**缺口换位**）。
300 帧全跑的 title 半验证=已执行前缀零记账 + 全 31 项臂化契约断言（dispatch 契约测
逐项钉）双证；完整 300 帧零记账验证待 task-30 B31B 落地后随第三轮补全。

## 3. 执行序证据

- boot（00 InitializeAfterReset 臂）→ 01 段0（APU 标志面+ScreenOff spin）→
  spin 唤醒帧 NMI 声分派 → **B31B 引爆**。
- title 分发面全程 armed（无 unported 记账）——contract 测试 `dispatch_arms_and_unported_faces`
  + `placeholder_arm_records_divergence`（终形账清断言）机械可检。

## 4. 缺口地图现状（B 跑行集账）

| 行 | 状态 | 归属 |
|---|---|---|
| ~~bank00:80D0 ×297~~ | **清** | task-23 批6b（01 深路径臂） |
| ~~bank00:82ED/90BA/939E/911A/9147 等~~ | **清** | task-23 批6b（12/15/1B/1C/17/18） |
| **bank07:B31B** | **新顶行** | task-30（声引擎分派层——在途） |
| （预期）B31B 落后 | handler 细行 | 音频全量路线图（1-2 周段） |

## 5. 复现命令

```bash
cargo run -p game-oracle --bin l3-dump -- --scenario boot-title
# → thread 'main' panicked at crates/game-core/src/frame/nmi.rs:184
#   unported @ bank07:B31B
```

【分类】A 级（执行实录+首分歧表址）；【置信】93
