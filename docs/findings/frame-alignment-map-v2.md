# 分歧地图 v2——matched-seam（spin-exit 缝对齐）实测图谱（frame-alignment-plan Step 0 产物）

- 日期：2026-10-30
- 性质：**A 级实测**（双侧原始行直接比对，零手工帧算术；全部数字可用 §5 命令复现）
- 前置：`docs/findings/frame-alignment-plan.md`（方案与 F1-F10 结论）；本档回填其 §3
  定数并**修正 F2 的一处归因**（§4）
- 仪器变更：`cases/case-crawlprobe.txt`、`cases/case-fcprobe.txt` 各加一行
  `sample_point spin-exit`（emu 腿采样点 endFrame→$C0D3 出口，与 rust 腿缝同相）；
  frames/press/capture/watch 未动一行（像素面不变——方案 F7）。

---

## 1. 仪器验收

| 项 | 值 |
|---|---|
| `analysis/traces/case-crawlprobe.run.json` | `"sample_point": "spin-exit"` ✓ |
| `analysis/traces/case-fcprobe.run.json` | `"sample_point": "spin-exit"` ✓ |
| emu 行数 | crawlprobe 943（endFrame 时代 960）；fcprobe 445（旧 460） |
| rust 行数 | crawlprobe 952；fcprobe 452（缝=架构恒定，未变） |
| L3 门影响 | 无——crawlprobe/fcprobe 不在 `oracle/traces/l3_manifest.json`；L3 门用例
  （sc-* 系）已在 spin-exit（task-24 3b 先例）；case-001 系=纯像素锚用例，按方案不动 |

## 2. 行集总表（对拍分母：缺行区间集合双侧对照）

| 区间 | emu 缺行（C0D3 未执行） | rust 缺行 | 归因 | 处置 |
|---|---|---|---|---|
| f1 | 有行 (0,0) | **缺**（PreSpin 不采） | boot 行簿记差（F2） | Step 1 |
| f2 | **缺**（0x00 臂 bogged 1 窗——**N₀=1 实锤**） | 有行 (0,0)==emu f1 | boot 行号错位的 rust 侧投影 | Step 1+2 |
| f3 | (FC1,TD9) | (1,9) | **双侧全同**（boot 行差在此已吸收） | — |
| f4-6 | **缺**（0x01 臂 3 内窗=2 旋+1 溢出——**n₁=1 实锤**，H5 证实） | 多 3 行 (1,9)/(1,8)/(1,8) | 0x01 臂 tick 泄漏缝行（F10） | Step 3+4 |
| f7-310 | 行集全同；TD offset **+1 自 f7 起** | | **+1 诞生点=0x01 窗**（§4 修正） | Step 4 |
| f311-313 | **缺**（0x1B 臂 3 内窗） | 多 3 行 (49,3)/(49,2)/(49,2)（FC 冻结生效、TD 在 f311/f312 各多减） | 同上 | Step 3+4 |
| f314 | (50,4) | (50,1) | **TD offset 1→3 跳变行**：窗内 emu 1 减（5→4）vs rust 3 减（4→3→2→1） | Step 4 |
| f352-358 | 缺（game-start MoreInit 7 窗） | **缺（同区间）** | **缺行区间双侧已重合——S5 行集目标已达成** | 无需动（F6 证实） |
| f315-915 | 行集全同；TD offset +3 恒定 | | MR 差仅 f793-795；T3/SS 差=T3 纪元投影（F5/F9） | 随上游自动闭合 |
| f916-917 | **缺**（冻 2 窗，FC 冻 132、TD 冻 0） | 多 2 行 (133,6)/(134,5)（FC 照常递增） | emu 侧未建模冻结（F8 开放项） | Step 5 子步 |
| f918-959 | FC offset **−2**（rust=emu+2，42 帧） | | f916-917 的下游投影 | 随 F8 |
| f960 | 缺（endFrame 960 截断） | 有行 | 采集截断边界伪影 | 记账（不修） |

## 3. TD/FC offset 轨迹（emu−rust，mod 256；回绕行显示为负模值属单帧伪影）

```
TD:  f3=0 → f7=+1（0x01 窗） → f314=+3（0x1B 窗 +2） → f918=+5（f916-917 冻结 +2）
FC:  f1-f915 恒 0（r9 title_no_return 修复生效的机械终验）→ f918 起 −2（42 帧）
```

0x1B 窗原始值（FC,TD 双侧）：

```
f310: emu(49,5) rust(49,4)   ← 入窗 offset +1
f311: emu 缺    rust(49,3)   ← rust 臂 tick：FC 冻结✓ TD 多减
f312: emu 缺    rust(49,2)   ← 同上
f313: emu 缺    rust(49,2)   ← 无减 tick
f314: emu(50,4) rust(50,1)   ← 出窗 offset +3（emu 1 减 vs rust 3 减）
f316: rust TD=9 回绕（T3 22→21）；emu f318 TD=0、f319 起回绕——T3 纪元差之源
```

0x01 窗原始值：

```
f3:  双侧 (1,9) 全同
f4:  emu 缺        rust(1,9)  ← cursor0 tick 泄漏（复制 f3 态）
f5:  emu 缺        rust(1,8)  ← 臂 tick TD 多减（emu 整窗无 prelude）
f6:  emu 缺        rust(1,8)
f7:  emu(2,8)      rust(2,7)  ← 出窗 offset +1（emu 1 减 vs rust 2 减）
```

## 4. 对方案的更正与证实（原地补注已同步 plan 档）

1. **【更正 F2 归因】TD +1 不生于 boot**：boot 差是纯行号簿记（rust f2 内容==emu f1），
   f3 起行号与状态双对齐（FC offset 0 直至 f915 为机械反证——若 boot 有相位差，FC 不可能
   全同）。**+1 实际诞生于 0x01 臂窗（f4-f7）：rust 臂 tick 多跑 1 次 UpdateTimer**。
   ⇒ Step 1（boot 行内联）只修行集形状；TD +1 由 Step 4 的 0x01 臂重构（内窗无 prelude）
   闭合。总 TD +3 的分解修正为：**0x01 窗 +1（Step 4）+ 0x1B 窗 +2（Step 4）**——Step 1
   对 TD 相位净贡献 = 0。
2. **【证实 F6/S5】game-start 缺行区间双侧已重合**（[352-358]×7 窗），TD offset 跨窗恒
   +3（f349-351 与 f359-362 同值）——净贡献 0。Step 5 的门序列调整**无必要**（除非
   trace 金腿因他步回退）。
3. **【证实 F3/H5】0x00 N₀=1、0x01 内窗 3（=2 旋+1 溢出，n₁=1）、0x1B 内窗 3**——§3 表
   「定数待图」三处全部回填。
4. **【证实 F10】0x01/0x1B 臂 tick 泄漏缝行**（f4-6、f311-313 共 6 行）；FC 冻结在臂内
   生效（f311-313 FC=49 恒）但 TD 不冻——**现役 continuation tick「无 prelude」性质不
   成立**（TD 在 f311/f312 各减），Step 3 的谓词单源化必须连 prelude 抑制一起补。
   > 【2026-10-30 更正（实施期复核）】「continuation tick 跑 prelude」表述不准：TD dec
   > 实来自**非续段拍**——0x1B 窗的 3 dec=转换检测迭代 prelude（tick310）+臂入口 cursor0
   > 拍 prelude（tick311，cursor==0 不算续段）+完成折叠迭代 prelude（tick313）；续段/spin
   > 消费 tick 本就跳 prelude（mainloop.rs 既有）。真正的机制缺陷=**边界数不足**（臂被
   > 拆成多个「伪迭代」各带 prelude）——Step 4 臂重构（单迭代 dive+哨兵收尾）闭合。
5. **【证实 F8】f916-917 emu 冻结两窗**，FC 冻 132/TD 冻 0，rust 照常 → FC −2 投影 42 帧。
6. **【新】T3 首分歧 f7**（T3←8 的 0x02 FadeInDelay 迭代完成行 rust=f7/emu=f8——臂窗
   行集差的直接下游），f8 起即回归相等；其余 T3/SS 差集中在回绕边界行（相位伪影）。

## 5. 复现命令

```bash
# 逐字段对拍（入库工具；exit 0=绿/1=分歧/2=规格错）
python3 tools/frame_seq_diff.py \
  --emu analysis/traces/case-crawlprobe.frames.jsonl \
  --rust shots_rust/case-crawlprobe.state.jsonl \
  --field FC=FC:zp@0x2D --field TD=TimerDelay:zp@0x29 --field T3=Timer3:zp@0x2C \
  --field MR=MainRoutine:zp@0x1E --field SS=SamusStatus:iram@0x200
# 当前读数：可比 942；FC 42 帧（f918-959）/MR 3（f793-795）/SS 15（f493-795）/
# T3 163（f7-795）/TD 941（f7-959）——全部为本档 §3 轨迹的逐行投影

# 行集（缺行区间）——一次性只读核（§2 全部数字来源）
python3 - <<'EOF'
import json
def rows(p):
    return {r["f"]: r for r in map(json.loads, filter(str.strip, open(p)))}
emu=rows("analysis/traces/case-crawlprobe.frames.jsonl")
rust=rows("shots_rust/case-crawlprobe.state.jsonl")
E,R=set(emu),set(rust); M=max(E|R)
# 断言本档 §2 核心结论
assert sorted(E-R)==[1] and sorted(R-E)==[2,4,5,6,311,312,313,916,917,960]
assert sorted(set(range(1,M+1))-E)==[2]+list(range(4,7))+list(range(311,314))+list(range(352,359))+[916,917,960]
assert sorted(set(range(1,M+1))-R)==[1]+list(range(352,359))
print("map-v2 行集断言全过")
EOF
```

## 6. 假设区（本档不含 C 级结论；f916-917 来源候选仍按 plan §7 H1 记录）

---

## 7. 收敛终态（2026-10-30 实施后实测——Steps 1-4 全落地）

修复全部落地后的 matched-seam 终态（同 §5 命令复现）：

| 面 | 终态 |
|---|---|
| 行集 | emu_only=∅；rust_only={916, 917, 960}——**全部对拍窗行集闭合**（916-917=F8 开放项、960=截断伪影） |
| fcprobe | **5 字段（FC/TR/MR/GM/T3）×445 可比帧全同**——FC 相位探针零分歧（本案源头判据闭合） |
| crawlprobe | MR/SS/SY/T3 **全同 0 分歧**；FC/TD 仅 f918-959（42 帧，=f916-917 冻结的下游投影）；SX 仅 f918-926（9 帧，同源） |
| 0x01 窗 | f3-f7 逐行全同（(1,9)/(2,8)/(3,7)…）；内窗 f4-6 双侧静默 |
| 0x1B 窗 | f310-f320 逐行全同（(49,5)…(56,8)）；内窗 f311-313 双侧静默；f314=(50,4) 单 inc/单 dec 精确落地 |
| game-start | f348-f365 逐行全同；缺行 [352-358] 双侧重合（F6 终验） |
| MR 窗 | f790-f800 逐行全同——**GameEngine -3 差窗（f793-795）消失**（F5 单因果链终验点闭合） |

落地形（与 plan §4 的差异点，均已 asm 复核）：
- 0x1B 臂=**dive 单迭代形**：head_hit 非独立迭代——同拍 jmp 直入臂 cursor0（face 新相
  位重建），臂 3 cursor 步+0xFF 哨兵收尾（单 prelude@w310+单 inc@哨兵窗=emu 1 dec/1 inc
  精确对形）。plan §4 的「入口拍在 w311」修为「入口拍=命中拍 w310」（内窗不可观测，
  两形同可观测；dive 形与 asm jmp 语义一致故取之）。
- 0x01 溢出窗=bogged 门（asm：旋①释放后 NMIStatus=1 残留⇒溢出窗 NMI 内块跳——
  prg7:251 同形；plan §4 iter_bogged(n₁) 原案成立）。
- engine AwaitNmi 臂补**门→span_left 装配**（首迭代声明门队列的同构面——缺此段则
  0x00 臂 bogged 门被当 tick 消费）。

---

## 8. F8 根因锁定（2026-10-30 实施期取证——H1 从 C 级升级）

f916-917 emu 冻结=**爬虫被击杀→爆炸初始化重活**（下行于行走路径分叉，非独立引擎缺口）：

- enemystat（$6B34-6B45）实录：slot5 d5 递减 B6(f913)→B5→B4→(冻 2 窗)→B3(f918)——
  重生延迟计数；f918 起 0x18 区新非零字节（01/01/04）=爆炸数据出现；
- 冻结 2 窗=死亡/爆炸初始化迭代的重活跨帧（无迭代完成⇒无 C0D3 行）；
- rust 无此冻结：其爬虫因**行走路径分叉**（map-v2 §7/爬虫档终判补注——f797 首步
  双轴对角+围小回路）不在弹道上（beam@f800 射出），无死⇒无爆炸⇒无冻⇒FC 照常 +1
  ⇒FC/TD f918-959 与 SX f918-926 的 42/9 帧投影全为该下游。
- **处置改判**：plan §4 Step 5 的「f916-917 按门序列补门」**作废**——先修行走路径
  分叉（转向核/碰撞检把天花板行走误判转角），路径闭合后双侧爆炸同帧，冻结随之
  同形（死亡链帧骨架如缺 2 bogged 窗再按图谱补，不允许预先手工补）。

> 【2026-10-30 更正（verifier 对抗复核轮裁定 fail——C.3 归因证伪，原地标注）】§8 的
> "F8 根因锁定/H1 升级"**不成立**，三处一手数据证伪：
> ① "slot5 d5 递减 B6→B3"标签错误——倒计数字节实在 $6B41=EnsExtra slot4+$0D
>   （jumpDsplcmnt，constants_ram.asm:1271；EnExtra 6×16B 自 $6AF4、slot4=$6B34），
>   非任何 d5；真 slot4/slot5 d5（$0445/$0455）f918 后继续 1A/5A 交替——爬虫活着。
> ② "0x18 区 01/01/04=爆炸数据"定位与语义双错——实在 $6B14/$6B1B/$6B22=EnsExtra
>   slot2 status/hi/type（**新敌槽激活**）；爆炸区在 EnExplosions $04C0-$04DF/$6BB4+
>   （constants_ram.asm:943-950/1273-1275）。且 rust 侧 f916 出现同形激活（仅差 FC
>   相位 2 帧）——双侧共有事件（首次横滚揭载新敌），非 emu 独有死亡签名。
> ③ 冻结与**首次横滚**精确同窗（SX 双侧 f915=0→f918 非零，f927 起同归 0D）——
>   回退至 crawler-corner-divergence.md:91 原假设（首次横滚 ScrollRight VRAM 重活）
>   为首选（B 级：同窗性实证+机理候选；横滚 VRAM 填充量级与 2 窗冻结吻合度待钉）。
> 域注：enemystat watch（$6B34-6B45）实为 EnsExtra slot4/5[0..1] 且与 Lua readRange
> 尾部 $6AF4-$6B73 重叠双读——本档 §8 误标温床，watch 改名/去重列为工具债。
> **不变项**：冻结=emu 侧行为、rust 未建模、FC/TD f918-959 与 SX f918-926 投影归因、
> "禁止预先手工补冻结窗（先修路径/横滚同形后按图谱补）"的处置——均不受影响。
