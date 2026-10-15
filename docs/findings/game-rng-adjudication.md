# findings：GameRng 算法定谳（task-20）——C000 闭式推导 + 三项 A 级收口

> 状态：**已证实**（2026-10-15，A 级三轨：asm 结构互核 × ROM 字节 + 种子序 + 动态交叉）。
> 交付面：`rng::RngState::step` 域实现 + `sub_GameRng` L2 族（8 案三方全等）+ N-0012 升 A。
> 复跑：`python3 tools/diff_subroutines.py`（L2 机器主检）+
> `python3 -c "…"` 单行复现（各节内嵌）。
> 本文件是 N-0012 升级的证据主载体；**假设区单列 §6，不混入结论**。

## 0. 结论摘要（判据对照 l3-gamecore-leg-verification.md §2 机械门）

| 交付 | 状态 | 证据级 |
|---|---|---|
| N-0012.level ∈ {A,B} | A（本 commit 升级） | A |
| N-0012.evidence 含 `bankNN:AAAA` 真址 | bank07:C000 / bank07:C0AC / bank07:C004-C011 | A |
| RNG 序列 L2 案 ≥1（sub_GameRng 族计数） | 8 案（l2_cases.jsonl `b-game-rng-*`） | A |
| RNG 面=全状态（严重-9 义务①-④） | ①序列等价②往返③调用计数④面外读写=0——见 §4/§5 | A |

## 1. ① C000 asm 考证（内容锚引证 + 结构互核注，链规 17）

m1disasm 内容锚：`refs/m1disasm/SRC/prg7_engine.asm:33` `RandomNumbers: ; 07:C000`。
**结构互核**（ROM 原字节 vs 反汇编逐条，26B 全对）：

```
bank07:C000  8A 48 A2 05            txa / pha / ldx #$05        （X 经栈保存）
bank07:C004  A5 2E 18 69 05 85 2E   lda $2E / clc / adc #$05 / sta $2E   ┐循环 ×5
bank07:C00B  A5 2F 18 69 13 85 2F   lda $2F / clc / adc #$13 / sta $2F   ┘（ldx #5/dex/bne）
bank07:C012  CA D0 EF               dex / bne $C004             （分支偏移 $EF=C004-C015）
bank07:C015  68 AA A5 2E 60         pla / tax / lda $2E / rts   （X 恢复；A=RN1 终值）
```

- **逐字节自证**：`python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:0x20010]; print(p[0x1C000:0x1C01A].hex())"`
  → `8a 48 a2 05 a5 2e 18 69 05 85 2e a5 2f 18 69 13 85 2f ca d0 ef 68 aa a5 2e 60`。
- **分支偏移自纠**：首核时按心算预期 `d0 f1` 判「不一致」——实为 `d0 ef`（BNE @C013，目标
  C004=C015-0x11）；**心算分支偏移是工具活**，逐字节以字节 dump 为准（否定结论换工具复核，链规 4）。
- 旁证：decode-sweep r3 窗口指纹「bank7 首 16B=`8A48A205…`」即本例程首 4B——两独立工作互证。
- **「注释≠语义」正面样本（第六案族）**：m1disasm 注「RandomNumber1 is increased by
  **#$19 every frame**」——实际字节是**循环内 +0x05 × 5 次**（=每**调用** +0x19）；且
  **调用≠帧**（§3 动态证 300 帧仅 295 调用）。注释是调用聚合的粗述，逐字节+动态才是语义。
  引用第三方注释时以此为鉴（D5：第三方注释同第三方命名，只作线索）。

## 2. ② 闭式推导（k=调用计数，**非帧号**）

每调用（循环×5 聚合）：

```
RN1_{k+1} = (RN1_k + 0x19) & 0xFF     （5 × 0x05；bank07:C004-C00A）
RN2_{k+1} = (RN2_k + 0x5F) & 0xFF     （5 × 0x13；bank07:C00B-C011）
A_out     = RN1_{k+1}                 （bank07:C017-C019 尾 lda $2E/rts）
```

种子（bank07:$C0AC `A9 11 85 2E A9 FF 85 2F`，boot 初始化序）：RN1_0=0x11、RN2_0=0xFF
⇒ **RN1_k=(0x11+0x19k)&0xFF、RN2_k=(0xFF+0x5Fk)&0xFF**。
周期：gcd(0x19,256)=gcd(0x5F,256)=1 ⇒ 双轴各 256、联合周期 256。
机器核：`rng::RngState::at_call(k)`（k=0..300 与逐调用 step 同一性单测
`closed_form_matches_repeated_steps`；周期钉 `period_is_256_on_both_axes`）。

## 3. ③ 动态交叉（运行态确证——Mesen2 三用途标签：**运行态确证**）

工件：`analysis/traces/boot-title.frames.jsonl`（300 帧逐帧 zp 快照；run.json 锚
scenario_sha256=`25b5206a…68c9`、generated_lua_sha256=`038e047e…3b0`、done.frames=300）。
提取 $2E/$2F 逐帧序列后对账（`python3 -c` 提取脚本随 commit 记录于 §3 复跑注）：

| 观察 | 结果 |
|---|---|
| f=1 初值 | (RN1,RN2)=(0x11,0xFF) = C0AC 种子——**boot init 动态自证** |
| 相异态转移次数 | 295（300 帧内；**call≠frame**——标题期跳帧，调用差集外无第三增量） |
| 转移增量集 | **调用差集（相邻相异态）={(0x19, 0x5F)} 单元素**（无其它改写者）；**全帧差集（含同态帧）={(0,0),(0x19,0x5F)}**——4 个零调 boot 帧（f=3,5,6,7）差值 (0,0)。口径分母不同勿混读（P20-F1，verifier 复审注 2026-10-15） |
| 闭式违例 | **0/295**（RN1_k/RN2_k 全合） |

复跑注（一步）：
`python3 -c "import json;rows=[json.loads(l) for l in open('analysis/traces/boot-title.frames.jsonl')];s=[(int(r['w']['zp'][0x5c:0x5e],16),int(r['w']['zp'][0x5e:0x60],16)) for r in rows];print(len(set(s)),all((b[0]-a[0])&0xFF==0x19 and (b[1]-a[1])&0xFF==0x5F for a,b in zip([s[0]]+list(s[:-1]),s) if a!=b))"`。

## 4. ④ L2 案机器主检（sub_GameRng 族，喂机械门谓词计数）

`oracle/subroutines/l2_cases.jsonl` `b-game-rng-*` 8 案（entry=bank07:C000，stops=[] 哨兵约定）：
boot 首调用 / 双轴回绕 / RN1 过零 / RN2 自最大 / X 保存恢复+Y 不动 / 入参 A 死输入对照 /
闭式 k=255 态 / 全零边界。**golden=闭式手推**（§2，算术机器核）**× ROM 执行独立裁定**
（staging 期即 golden==ROM 全对——双腿交叉验证后 game-core 腿补齐转绿）。
**面外读写=0（严重-9④）**：写面契约={$2E,$2F}——C000 全 26B 流只寻址此二址
（栈流量=机械不入面）；任一面外写都会打回 golden。
TDD 留痕：staging RED=「8 条用例, 8 失败（game-core 缺移植函数 sub_GameRng）」→
GREEN=「8 条用例, 0 失败」；突变敏感性=已知坏例（缺实现）8/8 全抓。

## 5. N-0012 升级记录（C→A）与全状态面口径

- N-0012 原 evidence「算法未证——zp RNG 面+取值例程待执行序引爆定位 bank:addr」→
  本文件 §1-§4 收口；**原主张原地保留于 note**（更正文化，不删历史）。
- **RNG 状态面={RN1($2E), RN2($2F)}=全状态**：步进例程读写面与钉名面互证，
  面外读写=0（§4 写面契约）；无寄存器暂存面外态（A 为返回值载体，X 经栈进出，Y 不触达）。
- **$8A MellowRandomNumber 分类**（消费侧派生暂存，**非 PRNG 状态机件**）：
  bank07:EE63 `SpawnMellows`（`lda RandomNumber1/adc FrameCount/sta MellowRandomNumber`）
  + bank07:EE86 `SpawnMellow`（`txa/adc MellowRandomNumber/and #$7F → y`、`adc RandomNumber2
  → x`、`rol MellowRandomNumber` 逐槽旋转）——由消费例程自 RN1/RN2/FrameCount 混合写，
  RandomNumbers 不触达。S5 快照面按**普通游戏状态**处理（需要时入 watch，不属 RNG 面）。

## 6. 假设区（C 级，不作结论）

1. **密码/掉落等消费例程的随机性观感**（m1disasm 注「password scrambling / item drops」）
   ——消费点语义未逐个考证（brinstar EnData08 混合式等）；本定谳只覆盖**状态推进例程**；
2. 「every frame」注的**意图**（作者本意是主循环每迭代一调）——只证事实（字节=循环×5、
   动态=295/300），意图属猜测面；
3. 新开 rng 专项场景双跑——lead 裁定「既有工件足够，双跑列可选随访」（verifier 索再补）。

## 7. 遗留

1. `frame/rng.rs` GameRng 桩的算法落地（S5 集成）归 porter——本定谳供语义源
   （N-0012 + 本文件 + rng::RngState 域实现）；互斥面未触；
2. 消费例程族（掉落/敌人移动/密码）的语义移植未排期（假设区 1）；
3. l3 机械门实施（enforcement① diff_traces 前置断言）属 gates 面（排期=§5 步骤 2b，
   l3 设计文档），非本任务。
