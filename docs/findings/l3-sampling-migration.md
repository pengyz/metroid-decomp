# findings：L3 采样迁移（task-24 3b/3c/⑤）——endFrame→spin-exit 全案记录

> 状态：**3b/3c/⑤ 已迁移**（2026-10-16）；② game-core 导出器=共享面待缝（§4）。
> 依据：task-13 契约 §5 实施序 3/3b/步 4（v5 四裁生效版）+ r12 遗留-1。

## 1. 采样点迁移（3b）

| 项 | 内容 |
|---|---|
| 旧采样点 | endFrame（帧渲染完成后）——「帧末 $1A」口径（**已作废**，见 §3） |
| 新采样点 | **spin-exit=$C0D3**（`@exitLoop_waitNMIEnd`，m1disasm prg7_engine.asm:218 注址直证） |
| 实现 | scenario_capture.lua `sampleTrace()` 函数化 + `$C0D3` exec 回调（callbackType.exec 真执行，r3 语义）+ once-per-frame 守卫；DSL `sample_point spin-exit` 声明（缺省=endFrame legacy，混态期两制并存） |
| 记录面 | run.json `sample_point` 字段 + l3_manifest `legs.oracle.sample_point`（per-leg 键，v5 §5.3c 混态禁置全局键） |

## 2. 换锚（⑤）+ 双跑

```
run1 sha256: 4f9ca79109d638de88301eaddffb576023ef770e5c79f7de71179c869a90ff38
run2 sha256: 4f9ca79109d638de88301eaddffb576023ef770e5c79f7de71179c869a90ff38
双跑逐字节一致: ✓   行数: 295 295
```

- oracle_sha256 全量换锚：`4655a986…`（endFrame 面）→ **`4f9ca791…`**（spin-exit 面）；
- scenario_sha256 换锚 `1865bfd0…`（boot-title.txt 增 sample_point 声明+口径作废注）；
- run1/run2 对称命名（P12-F3 约定）；归档 glob 顺手修 `.run1./.run2.` 排除
  （对照件被吞自反三案收口——scenario_run.py `archive_existing`）。

## 3. 口径作废明示（⑤）

**历史「帧末 $1A」口径自本迁移起作废**：帧末采样处 $1A 处于清 0/置 1 交接窗
（相位敏感面——实测帧末 $1A: 0×598+1×2，恰落「NMI 写 1 后、主清 0 前」相位）；
spin 出口 $1A **恒 1**（成环标志=spin 出口同源，退出=1）——3c 恒 1 断言自此可真启用
（门控：legs.*.sample_point=="spin-exit"，diff_traces check_nmi_anchor 在位）。

## 4. 行集事实 + 零回退验证

- **行集**：295 行（f=1,3,7..299）——boot 相入主环前缺 f=2,4,5,6；尾帧 300=spin-exit
  落 DONE 截断后。**diff 行集口径已随迁移改口**（diff_traces：spin-exit 腿按
  `legs.oracle.rows` 校验，endFrame legacy 腿仍按 frames 全集）；
- **零回退**：`gates/cli.py trace` 全绿（输出原文：「oracle 腿已验（sha256 对锚、
  295 帧、逐帧字段合法）…1 条用例, 0 失败」）；
- ③ R2 残余核查：diff_traces per-leg 键消费/混态禁置守卫/恒 1 门控注全数在位
  （259-295 行），docstring 无「帧末采样」残留措辞——③ 收口。

## 5. ② game-core 导出器（spin 出口采样）——共享面待缝（显式未完）

l3-dump 现状=**tick_frame() 末 watch_snapshot()**（tick 内序：NMI 尾写 1→唤醒→帧工作→
清 0→spin yield；tick 末快照落在清 0 后=**旧口径同病**）。spin 出口采样需 tick 内缝
（NMI 后/唤醒点快照）——game-core 无既有钩子（tick_frame 一体）；
**请求 game-core 侧最小缝**（porter 域，共享面通报制）：`tick_frame` 内唤醒点后
快照回调或 `spin_exit_snapshot()` 半步 API。缝就位后 l3-dump 采样迁 §1 同点+
dump_manifest 落 `sample_point=spin-exit`+恒 1 断言 B 跑实测。

## 6. 遗留

1. ②（上节）——L3 首跑 B 的最后前置；
2. 行集对账（game-core 腿 295 行同源对齐或显式排除缺帧行）随 ② 一并；
3. samus-move3 场景迁移（首批双场景之二，②后随迁）。
