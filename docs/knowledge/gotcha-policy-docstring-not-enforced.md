---
type: gotcha
---

# gotcha：政策写在 docstring ≠ 判据落到代码（判别力自检抓「文档声称与实现分离」）

## 现象
decode-sweep v2 的模块头写明「BRK/非官方操作码一律拒流」，但代码里 BRK(0x00)
仍在官方长度表中被放行——**文档政策与代码实现分离**。负对照（真数据指针表
$FC5D `00 FB FB FE FB FB 02 05`）的流穿过 `00` 字节漂进相邻代码达到 RTS，
被判「code」——若无负对照，这是一条会进结论的假阳性。

## 教训（链规⑤「测试器先过证伪」的实战抓错案）
- **声称的判别力要先被坏例验过**：正对照（已定谳缺口）+负对照（已定谳数据）成对进
  回归基准，缺一不可——单向正对照只能证「抓得住」，不能证「不乱抓」；
- docstring/注释里的政策**不是判据**：判据必须是可执行断言/分支（`if op == 0x00`），
  文档只作说明；「规则写了但没实现」与「规则不存在」在运行时不可区分；
- 与 gotcha-evidence-grep-needs-fixed-string 同族镜像：**可复现性/判别力的声称本身
  要可复现**。工具每轮自跑基准（BASELINE: PASS/FAIL + exit code）即为此设。

## 复现（2026-10-15，修复前）
```sh
python3 tools/decode_sweep.py   # 修复前：负对照 $FC5D verdict=未决（盲扫 0.625）→ BASELINE: FAIL
grep -n 'BRK 不作代码流' tools/decode_sweep.py   # docstring 有此句
grep -n 'if op == 0x00' tools/decode_sweep.py    # 修复前 0 命中（政策未落码）
```
修复：sweep_linear 显式 `if op == 0x00: fail`；修复后负对照 4/4 → data，BASELINE: PASS。

## 高危同族
- 任何「XX 一律拒绝/必须」类工具政策：grep 其执行点是否真有对应分支；
- 环⑤基线的负对照必须来自**已定谳的坏例**（本例 4 个真数据岛均带 xref 消费者证据）。
