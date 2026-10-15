---
type: gotcha
---

# gotcha：listing 上 grep 字节模式 ≠ ROM 扫描（数据岛分行导致假阴性）

## 现象
在 seed listing 里 grep 字节序列（如 `20 62 F1`）得 0 命中 ⇒ 判「无调用点」。
实际数据岛区域**每字节单独一行**显示（`F162  B9  $B9`），多字节指令序列在文件中
永不连续 ⇒ grep 恒 0，是**扫描盲区假阴性**而非事实。

## 教训（与 r2 brace 盲区同族）
- xref/调用点/字节序列结论必须以 **ROM 原字节扫描**为准（python 解析 prg 字节）；
- listing grep 只作线索；**反证性的 grep（证明「不存在」）尤其危险**——命中 0 有
  两种成因（真不存在 / 扫描盲区），必须换工具复核后才许下否定结论。

## 复现
首例：state-fixture 同期 batch-1——`grep -c '20 62 F1' reference/seed/metroid.asm`=0，
而 `prg[0x1F109..0x1F10C]=20 62 f1`（bank07:$F109 JSR $F162 实存）。
详见 docs/findings/seed-decode-gap-f0a2-f148.md。
