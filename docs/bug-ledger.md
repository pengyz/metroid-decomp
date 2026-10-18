# Bug 台账（docs/bug-ledger.md）

> 用途：登记**原版（ROM/oracle 行为）的 bug 与 quirks**。按 AGENTS.md D4/§6：
> Stage A **保真保留**原版行为，只登记不修；代码侧以 `// BUG-xxx: 保留原行为` 引用；
> 差分测试遇到已登记 bug = "已知偏差"，基线显式 TOLERATED 并注明 BUG 编号。
>
> 内容维护归 **verifier**（master-plan §4）；本 stub 由 captain 于 2026-10-12 创建
> （轮次日志 round-007，t3 门禁 docs-links 预扫描发现 AGENTS.md §6 引用本文件缺失，
> 裁定方案 A 补齐脚手架）。

## 登记格式（AGENTS.md §6）

```markdown
## BUG-xxx: <一句话现象>
- 现象: <可观察行为描述>
- 复现: <scenario/输入序列/ROM 偏移>
- 影响: <对差分验证/玩法的影响>
- 证据: <trace id | bank:addr | rom:offset | shot:file>
- 处理: Stage A 保真保留（代码注释引用 BUG-xxx）；Stage B 修复计划：<无/说明>
- 登记: <日期 / 登记人>
```

## 台账（当前为空 —— 尚无已证实的原版 bug 登记）

<!-- 新登记追加在下方，按编号递增；证伪不删历史，原地标注日期更正（AGENTS.md §3.6） -->

## BUG-001: morph×楔交互=游戏态损坏（radius 0 退化+位置冻结）

- 现象: morph 球态行进撞楔（球盒 16px 跨 8px 瓦壁——$5D 类）后 un-morph——
  status=8（站立）但 radiusY/radiusX=0 退化+位置冻结（speedY=$FD 持续上推）——
  Samus 卡死不可动；
- 复现: sc-elevator-ride f3380-3750（morph×楔交互）——
  morph 触发（x>120 down）→球滚→撞楔（$5D 壁 x 143）→UP 脱球→状态损坏；
  探针实录：stat($300) 7→8/radiusY($301)=0/radiusX($302)=0/speedY($308)=$FD/ScrY($52)=200 冻结；
- 影响: **差分验证需登记为已知偏差**（TOLERATED+BUG-001 引）——game-core 移植
  面需保真复现（Stage A 保留 quirk——D4）——玩法影响=楔区 morph=永久卡死；
- 证据: 三环探针输出（docs/spec/map-connectivity-graph.md §38）；
  SetSamusRoll/SamusRoll/ObjectCheckMoveUp 指令流（07:D0B5/D0E1/E7A2）；
- trace id: analysis/traces/sc-elevator-ride.frames.jsonl f3380-3750（探针窗）；
- Stage 处置: **Stage A 保真不修**（D4）——代码侧注 `// BUG-001`（porter 移植时引用）；
- 登记: 2026-10-19 decomp（lead D4 台账令）
