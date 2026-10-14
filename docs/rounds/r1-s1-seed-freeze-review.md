# rounds/r1 — S1 seed 冻结 · 独立对抗复核（2026-10-14）

- 被审对象: `c416d34` feat(ref): S1 seed 冻结
- 审核方: 独立审核员（verifier 角色，只读，证伪立场，scratch 副本独立重导证据）
- **verdict: pass** —— 字节忠实性、确定性、冻结一致性、门禁状态全部独立复验通过，可进入 S2

## 独立复验摘要（未采信实施方数字）

| 项 | 结论 | 复现方式 |
|---|---|---|
| seed 全量忠实性 | ✔ 8 bank 重建 131,072 字节 == ROM PRG | 审核方自写正则解析 seed hex 列重建（非 seed_build 代码） |
| 随机抽检（rng seed=verifier-20261014） | ✔ bank03:$B113、bank04:$BF63 逐字节 match | 独立选点 |
| 断言非空转 | ✔ 篁改 bank00.json 一字节 → AssertionError | scratch 篁改实验 |
| 确定性 | ✔ 两次重建 sha 均 b4f0c5e4…3383 | scratch 重跑 |
| 冻结一致性 | ✔ sha+行数与 baseline seed 段一致，seed 门禁 PASS | sha256sum / wc -l |
| 向量偏移修复 | ✔ 0x1FFFA=file 131082=文件尾 6B，解码 NMI=$C0D9/RESET=$FFB0/IRQ=$FFB0 | 独立计算 |
| 纪律字段 | ✔ c416d34 含 Evidence/Confidence；第三方符号名 0 混入（grep 机械名 1912/999/53） | git log / grep |

## findings 与处理裁定

| # | 分类 | 内容 | 处理 |
|---|---|---|---|
| F1 | 纪律违规（轻微） | seed 头部 WINDOW_EVIDENCE 含社区区域角色标签（bank0=Title/EndGame…），属第三方语义信息混入机械轨注释、无自证指针（AGENTS.md §3.7 边界情形；符号名本身干净） | **采纳审核方备选路径：S3 以房间/trace 证据回填指针后再恢复断言语气**。冻结件依 D3 不为注释改动而重生成；此条挂 S3 待办（见 master-plan S3 栏），届时证据化措辞经正规 seed 重生成通道一次性落地 |
| F2 | 纪律违规（轻微，已自愈） | baseline.json 曾手工改动，被 sha 链检出并记 tamper_recovered | 机制按设计工作（gates/cli.py 机器字段）；**此后基线变更一律走 `gates/cli.py baseline update`**；seed 段登记属该命令覆盖外字段，登记动作必须与 update 同批执行并接受 tamper_recovered 留痕 |
| — | 信息性提示 | items.kind/funcs/xrefs 为线性扫描机械产物，含固有噪声（如 bank03:$B113 切为 1 字节 data） | S3 建 trace 纠偏流程；完备性矩阵符号面若以 funcs 为初始分母，标注"机械切分待纠偏"（管线 R2 已登记） |

## 遗留假设

无新增 C 级项。
