---
type: gotcha
title: 硬链快照共享 fcntl 锁 inode——锁隔离必须锁文件独立 inode
date: 2026-10-17
evidence: 事故③根因修复通报（lead）；docs/findings/incident2-defense-acceptance.md 硬链注
tags: [flock, fcntl, inode, 硬链, 并发, cargo-lock]
---

# gotcha：硬链快照=锁面照旧（fcntl 锁按 inode 不按路径）

**现象**：`cp -al ~/.cargo /tmp/cargo-home-decomp` 做锁隔离快照——**锁文件
`.global-cache` 与原件共享 inode（link=5）**→ fcntl 字节段锁按 inode 记账 →
独立 CARGO_HOME 路径下**锁照旧互撞**（「跑验证必卡死」事故③真因）。

**教训**：
1. **fcntl/flock 锁按 inode 不按路径**——路径独立≠锁独立；硬链快照的锁文件=
   锁面照旧；
2. 快照后**锁文件必须断链**（删掉硬链让目标自建独立 inode）或全拷（cp -a）；
3. 验证锁隔离的判据=**双进程并发实测**（持锁×目标路径构建），不是路径存在性；
4. 我当时留注「共享 inode 写入面」——但漏了**锁 inode 共享**这层（写入注≠锁注，
   锁语义比写语义隐蔽）。

**修复形**（lead 事故③）：锁文件清→自建独立 inode→并发实测全绿→验证套件秒级
（4s/2s/0.1s/0.3s）。
**关联**：gotcha-unloaded-state-probe-misread（实验前提类同族）。
