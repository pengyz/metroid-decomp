# findings：decode-sweep 零头终谳（task-35，r1→r4 全线清账）

> 状态：**零头全清**（2026-10-17）。判据：双工具（nmikstas asm 内容锚×ROM 字节）+
> A/B/C 分级+字节级复现命令（nmikstas 标签/指令流逐段可查，复跑见各节）。

## 1. 终谳表（残余栏三类全收）

| 靶 | 范围 | 终谳 | 证据（A 级） |
|---|---|---|---|
| D-REG-07-048 未覆盖 | $FBCA-$FBDC（19B） | **code** | nmikstas LFBCA 指令流 19B 精确（ldx/jsr/lda/cmp/beq/sta/jmp——敌动画重置检） |
| D-REG-07-050 未覆盖 A | $FD08-$FD5E（87B） | **code+内嵌数据**（Table18 5B） | LFD08（距离计算）+LFD25（查表寻径）+**Table18 `.byte` 5B**+LFD5F——「疑非 C27C 表族」假说证实（确非 C27C 族=代码+小表） |
| D-REG-07-050 未覆盖 B | $FD84-$FD8E（11B） | **code** | LFD84 指令流（lda/and/beq/lda/sta/rts——敌状态旗判） |
| 007 尾 | $D1D7-$D1ED（23B） | **data** | 「Pointer table to code」10×`.word`（$CF55/$CC98/ExitSub/$D0B5…）20B+Table07 3B=**23B 精确** |
| 018 尾 | $DAD2-$DAD9（8B/尾 1B） | **data**（终判转 data） | Table0E 4B+Table1B 4B=8B 精确；消费者=LDAB0 `lda Table0E,y`/`ora Table1B,y`——原「基本定谳 code」**更正为 data**（asm 指令流边界=DAD2 函数 rts 止，其后纯表） |
| F438 尾 | $F438-$F43D（6B） | **code**（终判） | `jsr UpdateEnemyAnim`（=P12 边 $F438→$E094）+`jmp LF416`=6B 精确——P12 正面证据与 asm 指令流双证合龙 |
| F438 邻段 | $F43E 起 | **code** | LF43E 起连续指令流（敌动画计数族） |

## 2. 换时刻/双工具面

- 双工具：nmikstas 指令流/伪指令（表声明）× ROM 字节（P12 call_graph 边实测
  $F438→$E094 与 asm `jsr UpdateEnemyAnim` 合龙=动态×静态双源）；
- 换时刻：r3 动态收口的「0 执行/0 消费者」=路径受限（非否定结论）——本终谳以
  静态权威（asm 等价源）定判，动态面留移植后会师。

## 3. denoms v2 账面最终形

- **residuals 栏清空**（三类全转定谳——cells 2/tails 3 全收）；
- D-REG-07-018 定谳 **code→data 更正**（错面反向一枚：原判 code 的 8B 实为双表）——
  d_wrongface 计数面不受影响（018 原在 D 面基本定谳，非错面格）；
- 巨尾切分（v2_totals）不变；R6 慢帧分支未路由注照留（未观测≠不存在）。

## 4. 复现命令

```sh
sed -n '9098,9114p' refs/nmikstas-metroid-disassembly/Source_Files/Bank07.asm  # 048
sed -n '9271,9331p' refs/nmikstas-metroid-disassembly/Source_Files/Bank07.asm  # 050A
sed -n '9345,9350p' refs/nmikstas-metroid-disassembly/Source_Files/Bank07.asm  # 050B
sed -n '2946,2971p' refs/nmikstas-metroid-disassembly/Source_Files/Bank07.asm  # 007 尾（表区）
sed -n '4174,4186p' refs/nmikstas-metroid-disassembly/Source_Files/Bank07.asm  # 018 尾（Table0E/1B）
sed -n '8159,8160p' refs/nmikstas-metroid-disassembly/Source_Files/Bank07.asm  # F438 尾
```
