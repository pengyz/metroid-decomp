# R9 复审 verdict——03c72bfb（端到端火力假绿返工：双侧出弹实锤+可复现载体）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- 对象：返工 commit `03c72bfb`（针对 a1708910 复核 needs_revision 唯一项）
- **verdict：pass**

## 1. 非空转达成（双侧出弹）—— PASS（仓库工件实读）

- **case-shootprobe-probe**（重写后 press 700 a + 800 b，capture 810/815/825）：
  fire-10 双侧弹槽 **$03D0 全零→st=01/radius=02/02/onScreen=01**（双侧出弹 ✓），
  x=$1B **双侧同值**；残差=DieDelay（emu 03/rust 02）+$3DE 两字节=1 帧相位残
  （FC 域）——与声明同形。
- **case-001 f810 bullet-midflight**：同证（live $03D0，st=01/r=02 02/scr=01/x=1B；
  diffs 仅 $3DE/$3DF 两字节相位残）。
- L4 像素面转为**诚实 diff**：fire-10/15/25 = 329/343/395px（弹相位残），f815/f825
  同值复现（case-001 与 shootprobe 双 case 独立同数——互证）。
- f815/f825 槽区 $3D0-$3EF 双侧**逐字节全同**（死后态对齐；修前死态发散
  （radius FF/onScreen 0）——修前/修后对照在 findings §8）。

## 2. 证据可复现载体 —— PASS

- cases/case-shootprobe-probe.txt（新输入形）+ case-001 采点扩展均已入库；
- `python3 tools/l4_diff_run.py run-all` 一条命令双侧再生 .ram.hex（本复核即用其
  产物逐字节实读复现）；
- dumpPpu hex 大小写统一（rust {:02X}=lua %02X——r9 上轮注记①闭环）；
- findings §7（bank1-5 五处+bank7 窗反证原地注）/§8（复现载体链+修前修后对照）
  归档齐——上轮 needs_revision 两处方全部落实。

## 3. ratchet 撤集合规性 —— PASS（合规论证成立）

- 旧 shootprobe 5 点（fire-2/5/10/20 空槽假 match）随场景重写撤集：l4 闸按
  **scenario sha 键控**——新 scenario=新点集（旧点未锚定于新集），非 match 集回退；
  `gates check` l4 闸 **PASS**（闸自身裁定为准）。「match 只增不减」精神=防真回退
  掩盖——假绿点（空对空）撤销正是去假绿，方向一致。
- 新面诚实化：出弹点由假 match 转为**真 diff（相位残）**+槽态字节证据——符合
  「对拍只裁对不对」纪律（弹相位差=已归因时序族，非移植缺口）。

## 4. 必跑验证 —— PASS

- `cargo test --workspace`：**444 passed / 0 failed**（game-core 410 ✓）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。
- L4 复跑：**match 35 / diff 24（35/59）**——门链五点/落地三点/kick 修复值
  （626/498/531/490/732）全部保持。
- 观察注记（非阻塞）：f1000/f1150 diff 像素 11267/11256→11311/11300（+44）——
  双侧 fresh 重采下的一致小漂移，属 BUG-089 时序类采样相位方差（非新分歧类；
  l4 闸 PASS 在案）。建议后续记录该方差带。

## 裁定

| 项 | 结论 |
|---|---|
| 非空转（双侧出弹槽态实锤） | pass |
| 证据可复现载体（cases+工具+§8 链） | pass |
| ratchet 撤集合规（sha 键控+去假绿方向） | pass |
| 验证（444/0；gates 13/0；35/59 诚实化） | pass |

**verdict = pass**（a1708910+03c72bfb 任务闭环）。

Evidence: analysis/traces+shots_rust 双侧 f810/f815/f825 .ram.hex 逐字节实读
（shootprobe fire-10 + case-001 三点；三槽扫描+槽区 diff 清单）；shots_rust/matrix.md
（2026-10-29 fresh run-all 35/59）；cases/case-shootprobe-probe.txt（新输入形）；
findings §7/§8；cargo test（444/0）；gates check（PASS=13 FAIL=0）。
