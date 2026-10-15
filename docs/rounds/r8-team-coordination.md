# rounds/r8 — lead 协调账本（跨线事故/裁定/规程实录）

> 多 agent 共享单仓的协调事件记录（amend 竞态、孤儿作业、锁竞争、路由裁定）。
> 账本所有权=lead；事件回指各域账本（r5/r6/r7）不重复其详。

## 1. git amend 共享分支竞态事故（2026-10-15，porter 自报·教科书级处置）

**时间线**（reflog 可证）：
```
95886e0  porter F172（5 文件；verifier 批 8 已按此 pass）
37b6c6e  decomp docs 指针校正 commit
         ↓ porter 执行 git commit --amend -- <5 路径>，但 HEAD 已是 37b6c6e
de9ba99  amend 将 37b6c6e 改写：message 被 F172 文案顶替
cd27f79  verifier 批 8 账本（引用链含 de9ba99）
```

**定损**：`git rev-parse` 双树哈希一致（a806a82d…，TREES-IDENTICAL）——内容零损失，
仅 1 条 commit message 被替换。**不做 history rewrite**（cd27f79 引用在途，重写断链）。

**37b6c6e 原 message 全文存档**（reflog 取回，2026-10-15）：
```
fix(docs): 指针校正——time-race gotcha 引用改指 r7-design-closure 教训补录节

原文引「r7-decode-sweep-review.md method 面」为悬空指针（教训原文实落
docs/rounds/r7-design-closure.md「教训补录（2026-10-15）：否定结论换时刻复核」节，
verifier 通报）。原地更正注保留历史主张；t3 行同步如实化（「声称记账≠记账」自省案）。
教训自用：建链前先核目标存在。

Evidence: grep -n -F '教训补录' docs/rounds/r7-design-closure.md → 239:## 教训补录
（2026-10-15）：否定结论换时刻复核（负向断言的时效竞态面）（目标实存后建链）；
python3 gates/cli.py check 输出原文：「[PASS] docs-links … 全部有效」「CHECK: PASS」。
Confidence: 95
```

**处置**：①原文本账本存档（本节）；②porter F19A commit body 带「de9ba99 message
失实」原地更正注（其自报方案，已批）；③教训→链规 14（见 prompts/README）。

## 2. 孤儿后台作业记录（2026-10-15）

`82d0d97b`（4 次状态翻转：failed×2/finished/closing）与 `347f8998`（failed）为平台
生命周期噪声——实为 teammate 执行体会话的信号翻转，产出均在仓内可验（decode-sweep
d058c40/e526ad4），无损失。记档不追；判据=工作树/门禁/账本三方核验。

## 3. index.lock 竞争热点实录（2026-10-15）

三连撞锁（porter F172/lead task-5/verifier 账本）均按链规 13「判活死→活锁退避」通过，
零删锁零丢失。锁热点=高频提交期+pre-commit 门禁持锁期（proof-checks 命中时 ~43s）叠加。

## 4. 路由裁定备忘

- R11-F3 一行修正：lead 自落（三路原则：能不跨界就不跨界）；
- 「否定结论换时刻复核」教训：verifier 通报→decomp 落 knowledge→指针校正——域间路由
  只动嘴不动手，符合写路径纪律；
- task-5 矩阵锁：协调锁协议（先宣布锁→短窗作业→发解锁信号）首用成功。
