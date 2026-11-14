---
type: gotcha
title: 定点文本替换会静默 no-op——「声称改了」与「diff 里有」是两件事
date: 2026-11-14
status: confirmed
---

# 定点文本替换静默失效：声明与 diff 不一致

**症状**：我在 commit message 与给复核员的回信里写了「文档 X 已改成 Y」，复核员实读文件发现
那一行**原封未动**；`git show <commit>` 里也没有对应 hunk。连续两轮被同一处挡住。

**根因（脚本层）**：用 `python -c`/heredoc 里的 `str.replace(old, new)` 做定点编辑时，若 `old`
与实际文本哪怕差一个换行位置或一个全角括号，`replace` **静默返回原串**——不报错、不提示。
我随后 `sed -n` 只看了**我以为**改的那几行，没看真正命中的位置，于是把「没改」当成「已改」。

**实例**：`docs/findings/elevator-evidence.md` 的 sem 段（2026-11-14，commit `1643309c`）——
旧串跨了三行换行，我按两行拼接写 `old` ⇒ 零命中；同 commit 里另外三处短串替换都成功了，
形成「部分成功」的假象，掩盖了零命中的那处。

**处置（现在固定这么做）**：
1. 定点替换必须 `assert old in s`（或 `s.count(old) == 1`）再替换；
2. 提交前 `git show HEAD -- <file> | grep -c '^[+-]'` 确认 hunk 非空；
3. 给复核员的回信只引用**已在树里**的文本，不引用「我刚写的」。

**更一般的形态**：这是本项目反复出现的「声明与证据不一致」家族的脚本版——
同族有「局部证据当全称证据」「断言自实现（测试复用实现常量）」。共同点：**结论与事实之间缺一次机械核对**。

**相关**：[[decision-gate-trust-root-boundary]]（判据与被判对象必须同类）、
`docs/knowledge/gotcha-*` 中的「报错先怀疑自己的读数脚本」诸条。
