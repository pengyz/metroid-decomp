---
type: gotcha
---

# gotcha：采样点即盲区——相位归属必须覆盖争议相位，回调内状态字段先验真值

## 现象
task-15 ④ 据「$1A 写点 scanline<241、NMI 零写」主张**同帧建模**——被裁定推翻：
NMI 尾 `ldy #$01 / sty NMIStatus`（prg7_engine.asm:279-280，nmikstas $C10B）**恒写
$1A=1**，是全库唯一非零写者。「NMI 零写」实为 **inputPolled 采样点未覆盖 NMI 相**
的采样盲区人工产物；scanline 相位归属全部错位（跨帧定案）。

双重叠加的错误：
1. **采样点选择错**：争议点是「NMI 相是否有写」，而相位归属用了与 NMI 相位无关的
   状态采样；
2. **状态字段真值未验**：回调内 `st["ppu.scanline"]` 与 pc 同族**回显/陈旧**
   （r3 v1 已证 pc 回显——同款字段却没做同款真值性检验）；
3. 静态扫描同案盲区：只扫 `85 1A`(STA zp) 漏 `84 1A`(STY zp)——**操作码覆盖面不全**
   的「写点扫描」给出「仅 2 写者」的假完备。

## 教训
- **采样点即盲区**：负向断言（「X 相无写」「NMI 零写」）前先问「我的采样面覆盖了
  争议相位吗」——与 gotcha-listing-grep-not-rom-scan（扫描盲区）/gotcha-negative-
  assertion-time-race（时效盲区）同族第三元：**空间盲区/时间盲区/相位盲区**；
- **回调内状态字段先验真值**：pc 已知回显 ⇒ scanline/frameCount/sp 等同类字段默认
  可疑，用前先对照已知真值（如 C0D9 命中时 scanline 应恒 241）；
- 相位归属用**事件序**（进入/退出标记之间的 seq 边界），不用回调内时钟字段；
- 静态「唯一写者」类结论按**全部写操作码族**扫（STA/STY/STX/RMW INC/DEC/ASL…）。

## 复现（2026-10-15，裁定记录）
```sh
grep -n "sty NMIStatus" refs/m1disasm/SRC/prg7_engine.asm   # 280: sty NMIStatus（前 279: ldy #$01）
grep -n "NMIStatus" refs/nmikstas-metroid-disassembly/Source_Files/Bank07.asm
# LC10B: STY NMIStatus  ← NMI 尾恒写 1（我方「NMI 零写」的反例即此行）
```

## 高危同族
- 任何「某相位/某窗口无事件」的结论（先核采样点覆盖）；
- 回调/中断上下文里的 getState 快照用于时序归属；
- 单操作码族的写点/读点扫描（STA-only 扫写点案）。
