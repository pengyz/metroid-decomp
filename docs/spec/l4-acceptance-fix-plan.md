# L4 验收修复方案 Spec v2（对抗复核修订版）

> 状态：v2（复核 verdict=fail 后修订；findings F1-F8 逐条处置）｜日期：2026-11-01
> 判定标准：**L4 逐格 match 集合**（用户裁定 2026-11-01）；L3 状态面为逐轮护栏（不回退）
> v1 复核记录：子 agent 证伪复核（8 findings + U1-U10 假设清单）——v2 逐条吸收

## 0. 分母与基线（可复现口径——F2/F7 修正）

| 面 | 分母 | 基线 | 复现命令 | 验收 |
|---|---|---|---|---|
| L4 视觉 | 97 格 | **待定谳**（见 §0.1）| `python3 tools/l4_diff_run.py run-all` | 97/97 match（或 §6 裁定后 95/97+2 TOLERATED）|
| L3 状态 | 230 点 | 9/230 | `cargo run -q -p game-oracle --bin l3-dump -- --scenario sc-first-room-full` + 扫描窗口 f900-3200 步长 10（共 231 采样帧，扣除缺 1=230）| ≥94/230 且逐轮不降 |
| 单测 | 429 | 全绿 | `cargo test --lib -p game-core` | 保持 |

### 0.1 基线定谳（F2——**已定谳 2026-11-01 14:17**）
- **定谳结果**：HEAD 新鲜 run-all = **match 54 · diff 49 · pass 54/103**（DSL 增量后 103 格=97+6 密化捕点）。
- **f500 定谳=✅match（0/61440）**——入库锚 `data/status/l4_ratchet_baseline.json` 正确；陈旧矩阵的 f500=diff 属实验产物污染 ✓ F2 关闭。
- 54 vs 推算 55 的 -1 差：6 密化捕点落 A 类倒置窗（预期 diff）+1 格回归待查（Phase 2 复测时顺带定谳）。
- **证据指针**：`data/status/l4_ratchet_baseline.json` + run-all mtime 2026-11-01 14:17:51（shots_rust/ 仅参考）。

## 1. 逐格分类账（42 格——F1 重导：每格一行，与 matrix.md 可对账）

> 【2026-11-01 勘误·Phase1③ 实测】frame_seq_diff 真差异窗=anim f1272-1699（246 帧）/
> status f1376-1699（171 帧）——**A 类窗 f1128-1200 的 anim watch 双腿全同** ⇒ A 类 L4
> 差=渲染面非状态面（倒置证据系 ram.hex 采集点状态，watch 面无差）；@HEXOFF=字节偏移
> （轮 26 假象「f360 起差」系单位误读 $500）。时间平移假说证伪（shift0 最优）。
> ⇒A 类根因改判「渲染/时序面（BUG-089 同类嫌疑）+f1272 起 anim 分叉」，Phase 2 入口
> =f1272 分叉帧定位。
> 【二次勘误】分析脚本偏移混用（0x205 字符形=垃圾）致「1680 全差」假象 ✗——工具
> @HEXOFF 字节形正确（@205=char 0x40A ✓ 交叉核 1434/1680 同）✓。f1272 窗实值：emu
> AnimResetIndex=14 vs rust=46（anim 表索引面差）。
> 【根定位（正解偏移后）】**anim 推进 4 帧相位差**：金 Salto(0x0E)→0x00 在 f1276 / 我
> 在 f1272（f1250-1271 双腿 0x0E 全同 ✓）——非表值差、非状态机倒置、非时间平移——
> **animDelay/表推进计时面**（DC8F UpdateObjAnim 的 delay 倒数/FF 回绕相位）。Phase 2
> 根修对象=anim 推进计时。
> 【DC8F 合规】UpdateObjAnim 全径对表（:5836-5870）：delay==0⇒推进/dec 后≠0⇒出/
> 落过⇒推进+reload(A)/表走 FF⇒reset 回绕——我方 update_obj_anim 同构 ✓✓。**金方
> animDelay 无 watch（数据缺）**——相位对账缺口：下步 DSL 加 watch 0304（animDelay）
> +0306（animIndex）重采双侧→相位对账→修。

像素差单位=差异像素/61440。类定义：A=状态机运行期 / B=镜头-滚动内容 / C=弹道-显示 / D=敌面相位 / E=其他已定位 / T=BUG-089 渲染-采样时序（记账容忍裁定候选）。

### morphball（14 diff）
| 帧 | px | 类 | 根因 | 证据 |
|---|---|---|---|---|
| f1128 | 117 | A | Stand↔Run 倒置窗（anim 相位）| 轮67 ram.hex 四面（f7e2a66b）|
| f1140 | 471 | A | 同上 | 同上 |
| f1152 | 349 | A | 同上 | 同上 |
| f1164 | 10042 | B | 倒置→移位级联（见 §1.1 级联声明）| 轮52 六面（6a839a45）|
| f1176 | 10024 | B | 同上 | 同上 |
| f1188 | 186 | A | 倒置窗尾 | 轮67 |
| f1200 | 110 | A | 倒置窗尾（另有 H1 专项考证——见 §4）| findings/l4-f1200-horz-momentum-divergence.md |
| f1300 | 21822 | B | 换房分叉（f1560 MPX/SDir）| 轮52 ram.hex 直比 |
| f1460 | 17418 | B | 同上 | 同上 |
| f1520 | 17494 | B | 同上 | 同上 |
| f1560 | 24832 | B | 同上 | 同上 |
| f1620 | 25300 | B | 同上 | 同上 |
| f1650 | 25300 | B | 同上 | 同上 |
| f1700 | 25300 | B | 同上 | 同上 |

### case-001（6 diff）
| 帧 | px | 类 | 根因 | 证据 |
|---|---|---|---|---|
| f810 | 300 | C | 弹道/显示（弹帧 0x5A 未视觉验）| matrix.md + 轮24（b88dbdfa）|
| f815 | 210 | C | 同上 | 同上 |
| f820 | 259 | C | 同上 | 同上 |
| f825 | 190 | C | 同上 | 同上 |
| f950 | 556 | E | 组合动作相位（待逐格定位）| matrix.md |
| f1200 | 1015 | E | 旋/解旋时机动画轴（**H1 待证**）| findings/l4-f1200-horz-momentum-divergence.md |

### case-002（3 diff）
| 帧 | px | 类 | 根因 | 证据 |
|---|---|---|---|---|
| f850 | 251 | B | 滚动内容 | matrix.md |
| f1000 | 264 | **T** | BUG-089：CHR/名表/属性/调色板四面全同⇒渲染/采样时序 | bug-ledger.md:60-69 |
| f1150 | 272 | **T** | 同上 | 同上 |

### 其他 case（19 diff）
| 用例 | 格 | px（已知）| 类 | 根因 | 证据 |
|---|---|---|---|---|---|
| scrollseam | 3 | 461/13449/14466 | B | 滚动内容（f500 态待定谳——§0.1）| matrix.md:120-122 |
| shootprobe | 3 | 284/236/244 | C | 弹道/显示（与 case-001 f810-825 同值同证）| matrix.md:124-126 |
| leftwalk2 | 5 | 含 f820=12100 | E | 逐格定位（f820 越界值已更正入账）| matrix.md:89 |
| crawlprobe | 2 | — | D | f1838 转角 d05 位差（bit6/bit2-1）| 轮32-42（7c46a4ef 等）|
| killenemy | 1 | — | D | 同上 | 同上 |
| collide | 3 | — | E | 逐格定位 | matrix.md |
| leftwalk | 1 | — | E | 逐格定位 | matrix.md |
| jumpledge | 1 | — | E | 逐格定位 | matrix.md |

**核销**：5(A morph)+2(B morph)+7(B morph)+3(B case002)+3(B scrollseam)+4(C case001)+3(C shootprobe)+2(D crawl)+1(D kill)+2(E case001)+5(E lw2)+3(E collide)+1(E lw)+1(E jumpledge) = **42** ✓（A=5、B=15、C=7、D=3、E=12、T=2（T⊂B case002 计数））

### 1.1 级联归因声明（F8）
morphball A→B 为**级联假设**（anim 倒置→移位→换房），时序先后≠因果——Phase 2 修复后须复测 B 格是否随动（是⇒级联成立；否⇒B 独立修）。D 类与 L3 f1838 d05 的**跨场景同根**亦为假设（U7）——Phase 5 逐格独立验证。

## 2. 修复阶段（每轮=行为代码+数字移动——F1 算术闭合）

| Phase | 内容 | 目标格 | 算术 |
|---|---|---|---|
| **1** 定谳+诊断增量（1-2 轮）| ①HEAD run-all 定谳基线+f500 ②DSL 增量（watch 加 $0305/$0306、capture 密化 f1128-1200）③frame_seq_diff 双腿逐帧+**输入相位对照**（按钮/Joy1Change/注入时点——F3）| 0（基建轮，§3.2 上限 2）| — |
| **2** A 类根修（2-3 轮）| 倒置触发帧→运行期 asm 路径→修 | A=5 | 55→60 |
| **3** B 类根修（2-3 轮）| ①级联复测（Phase 2 后）②换房/滚动三联动 ③BUG-089 两格走 TOLERATED 裁定（§6）| B=15（-2T）| →73 |
| **4** C 类根修（1-2 轮）| ①弹帧 0x5A 视觉验证（read_image——**至今未验**）②弹出生位相位 | C=7 | →80 |
| **5** D/E 类批量（2-3 轮）| ①占位臂/BUG 碰撞筛查先行（§4）②f1838 d05 ③E 类逐格 | D+E=15 | →95 |
| **6** 验收（1-2 轮）| run-all 97 match（或 95+2T 裁定）+L3 ≥94+gates all+长脚本+12 case 全复跑 | — | **闭合** |

**算术**：55+5+13+7+15=95 + 2T 裁定 = 97 ✓（若 f500=diff 则基线 54 起算、Phase 3 多 1 格，Σ 仍闭合）

### 2.1 失败分支（F6d）
Phase 2/3/5 修不动⇒该格移入 gap 表（显式）+Phase 6 走降级判据：`95 match + gap 表全登记 + 用户裁定 TOLERATED` 或 `不通过`——不得以"合规注释"充数。

## 3. 反空转硬约束（F6 重写）

1. **轮类型**：修复轮（行为代码+数字移动）/ 基建轮（Phase 1 专用）/ 验收轮（Phase 6）——**基建轮上限 2**，超出需用户批准；除基建轮外数字不动=无效轮。
2. **数字定义**：逐格 **match 集合**（对齐 `data/status/l4_ratchet_baseline.json` 的 match 只增不减）——像素差缩小**不算**数字移动；L3 same 数为辅助同口径（f900-3200 步长 10 的 230 点集合）。
3. **L3 护栏**：逐轮 L3 same 数不降（轮57/66 教训：L4 中性但 L3 崩）——L3 降⇒回退或修复。
4. 禁止 docs-only 提交（goal 明文）；注释随行为同 commit。
5. 每轮 AGENTS §4 三问写入 commit（Evidence/Confidence 必填）。

## 4. 缺口登记（F4 补全——显式）

| 缺口 | 来源 | 处置 |
|---|---|---|
| L4 42 格 diff | §1 | Phase 2-5 |
| **BUG-089**（case-002 f1000/f1150 渲染/采样时序）| bug-ledger.md:60-69 | Phase 3 走 TOLERATED 裁定（D4 记账容忍）——**不作为根修目标** |
| **f1200 H1 待证**（旋/解旋时机动画轴）| findings/l4-f1200-horz-momentum-divergence.md | Phase 5 先跑 H1 采证（emu f944-952 disp/animResetIndex） |
| f1838 d05 bit6/bit2-1 | mainloop.rs:549-568 | Phase 5 |
| F744 翻转位写面余项 | 769e50a3 | Phase 5 |
| Skree delay 1 帧相位 | mainloop.rs:557-558 | Phase 5 |
| toggle FC+1 的 L3 连锁 | 7a1dbf81 | Phase 2 真根修后重测去留 |
| **占位臂清单**：F6DF x 力、LoadElevator/LoadStatues/LoadPipeBugHole（room.rs:456）、spawn 类型（pickup/spawn.rs:64）、死亡链（physics.rs:919）、NoiseCont（nmi.rs:550）；gap_map 5 址（0xB7E7/0xCCD8/0xCE35/0xF6DF/0xFB7B）| 代码内+divergences.json | Phase 5 前逐格**占位臂/BUG 碰撞筛查**（42 格是否落占位臂路径）|
| R 规则面完备性 | D7 不可证 | 声明性覆盖 |

## 5. 假设清单（U1-U10 显式——F3/复核清单）

| # | 假设 | 处置 |
|---|---|---|
| U1 | 42 格全由 game-core 行为分歧导致 | **已证伪**（BUG-089≥2 格）——T 类分立 |
| U2 | 所列 asm 条目合规可外推全部静态面 | **限缩**：结论只在"已排除面内"成立；数据表/占位臂未排除 |
| U3 | 盘上产物=HEAD 行为 | **已证伪**（matrix.md 陈旧）——§0.1 定谳 |
| U4 | anim 倒置=状态机 bug 而非输入相位 | **未定**——Phase 1 输入对照判别 |
| U5 | 修复面只有行为代码 | **限缩**：渲染层（BUG-089 白名单）/harness 层（注入时点）修复有位置 |
| U6 | 42 格不经占位臂 | **未筛查**——Phase 5 前置筛查 |
| U7 | D 类 L4 格与 L3 f1838 同根 | **未定**——Phase 5 逐格独立验证 |
| U8 | L3 230 点口径可复现 | §0 已钉命令+窗口 |
| U9 | 97/97 无需白名单裁定 | **已证伪**（BUG-089 冲突）——§6 裁定径 |
| U10 | Phase 目标是下限且剩余格有归属 | §1 逐格映射已闭合（Σ=42）|

## 6. 裁定径（AGENTS §5 视觉白名单/TOLERATED）

BUG-089 两格（case-002 f1000/f1150）：Phase 3 提交用户裁定——①白名单（渲染/采样时序类）②TOLERATED+BUG-089 记账 ③继续根修。**裁定前不得计入 match**。

## 7. 证据指针汇总（F7 修正）

- 逐格数据：`data/status/l4_ratchet_baseline.json`（入库锚）+ `shots_rust/matrix.md`（仅参考，注明陈旧）
- A 类直证：f7e2a66b（轮67）+ analysis/traces/case-morphball-probe_f01128*.ram.hex
- B 类直证：6a839a45（轮52）
- 合规链排除面：fd4a876e/8f65adf7/1bc7361b/86e5eb0a/b844b937/a2948845/cfb1c25d/3440cf0f/53189c4c/f496a281/0bbc39ea
- L3 口径：`cargo run -q -p game-oracle --bin l3-dump -- --scenario sc-first-room-full` + f900..3200 步长 10 扫描
