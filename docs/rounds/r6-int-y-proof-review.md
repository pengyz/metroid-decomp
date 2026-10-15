# rounds/r6 — int_y 证明件复审账本（46691294）+ 3780 案复核记录

> 归档 int_y 证明件（8051ed0）对抗复审与其 3780 案独立模型复核——此前仅存 commit message
> （int-x 复审 F3 指出指针悬空），本文件补正并作为指针落点。

## 复审结论（46691294）

- verdict：needs_revision（E1 注释声称未运行的 10^6 桥接 / E2 假设台账缺失 / E3 spec 陈旧）
- **语义/证明五主张全部独立证实**，含审核员**自建独立 Python 语义模型**（未读被审解释器/port）
  生成 **3780 条 golden**（阈值边界积 + 400 随机 + 20 死输入探针）喂 l2-diff 全量三方 0 失败
  ——反「补偿性错误」三角闭合；gen-int-y 4 条 golden 逐一被独立模型复现（消 factory
  same-source 疑虑）。
- 审核员独立反汇编验证：E3A5-E3E4 恰 17 条主流指令 + D147 字节逐字节吻合；E3C7 BNE 恒跳
  （LDX 清 Z）等微结构核过；穷举机全量复跑 exit 0。
- E1-E3 已整改（efedd5e 同批/后续 commit）；整改复核见 int-x 复审 E 组实证。

## 3780 案资产

- 位置：`oracle/subroutines/generated/int_y_verif_3780.jsonl`（md5 5c1e527e… 与审核员资产
  逐字节同源）；活跃集抽样 30 条（keep-Y 15 + 非钳位 15）。
- **抽样口径（补记）**：keep-Y=「out.regs.y≠0」前 15 条；非钳位=「y=0 且 a∉{5,250}」前 15 条
  ——按生成序取前 N（int-y-verif 模型按边界积生成序），非随机。

## int_y 证明件的残余（承接 l2-proof-assumptions A 组）

- A5 int_y 规模桥接（解释器↔审计核）未做（int-x 侧有 66,976 点桥接）；
- 全量 2^32 穷举复跑由 int-x 复审方独立执行中（其预告核心面全立住）。
