# FC 相位残差考证——rust↔emu FrameCount 恒差的产生点与爬行残差归因（r9）

- 日期：2026-10-29
- 状态：**根因已钉（引擎级 +6，A 级逐帧实录）**；修复=移植错路由 porter（本档不修）
- 载体：`cases/case-fcprobe.txt`（FC watch 探针，与 case-001 前 420 帧同相）

## 1. 现象（已证实）

case-001 各采集点双侧敌槽（slot4/5，镜像爬行）净位移不同：emu x +12（BC→C8）、
rust x +4（BC→C0）@f825；且轨迹形态不同——emu 两波 +8/+4 前进+y 加速下爬，
rust +4 后冻结并于 f810-825 出现 −1 回摆（转角回转）。f420–f735 双侧敌均静止
（300 帧，可见性门控），分歧集中在 f735–f825 窗。

## 2. 直接原因（已证实，A 级）

**FrameCount($2D) 引擎级恒差 +6**（case-fcprobe 双腿逐帧对拍，452 可比帧）：

| 窗 | emu | rust | 差值演化 |
|---|---|---|---|
| 上电初始化 f4–f6 | FC 冻在 1（3 帧） | 逐帧 +1 | +3（f4–f6） |
| title→menu f311–f313 | FC 冻在 49（press310 后 3 帧） | 逐帧 +1 | +3（合计 +6） |
| menu→game f352–f358 | FC 冻在 87（7 帧） | 状态行缺，93→94/8 帧（同步冻） | **+0（双侧一致）** |

- capture 面表现 +7（case-001 f420–f825 六点恒差 +7）：= 引擎 +6 + rust 腿
  capture 在状态行之后沿读一拍（采样惯例差；emu capture==其状态行同值）。
- 差值全程恒定（无漂移）——462 帧内 9 个变化点全部对应上述两窗的阶跃。
- 复现：`python3 tools/l4_diff_run.py run case-fcprobe` →
  `analysis/traces/case-fcprobe.frames.jsonl`（emu 逐帧 FC）+
  `shots_rust/case-fcprobe.state.jsonl`（rust 逐帧 zp，$2D=偏移 0x5A）；
  或 `align case-fcprobe --frames 459 --emu-trace …`（align 判裁拒跑为预期——
  两侧 watch 面不对称；state 文件已产出即够本考证）。

## 3. 传导链（已证实，asm+代码读证）

1. 爬行 6/8 步门 `CrawlerAIRoutine_ShouldCrawlerMove`（07:F852，prg7_engine.asm:11512）：
   `txa / lsr lsr lsr / adc FrameCount / lsr / rts` + 调用方 `and #$03 / beq @exit`
   ⇒ 门=((槽偏移>>3 + FC)>>1)&3，0=不动。
2. 槽偏移语义直证：`GetEnemyData`（07:EB0C，prg7_engine.asm:8740 区）房间数据
   `and #$F0 / tax` ⇒ **PageIndex 存槽字节偏移（slot×0x10）**，asm `txa/lsr×3`=2×槽号
   ——rust `crawler_ai`（mainloop.rs:866）`(x >> 3 + FC) >> 1 & 3`（x=偏移）**逐位同构**
   （考证中曾疑 rust 槽相位项为移植漂移，经此 ROM 证据**撤消**，原地留档）。
3. FC 差 +6 ⇒ (FC+6)>>1 = (FC>>1)+3 ⇒ 门值 &3 相位 +3（半周期）⇒ 移动窗错位、
   **转角时机错位** ⇒ 环爬相位差（同一周期环、不同相位 ⇒ 采样时刻净位移不同：
   emu +12 vs rust +4 是相位差读数，不是速度差）。
4. 同族暴露面：一切 FC 门控逻辑（敌动画 3 帧门 UpdateEnemyAnim 等）在 f420 后
   全部错 3 个 FC 子相位。

## 4. 分类路由（verifier 模板）

```markdown
## diff-case fc-phase
- 输入序列: cases/case-fcprobe.txt（前 420 帧与 case-001 逐帧同相）
- 现象: 引擎级 FC 恒差 +6——上电初始化（f4-f6）与 title→menu（f311-f313）两窗
  emu 各冻结 3 帧、rust 未复刻冻结（menu→game 窗双侧一致冻结，无差）
- 分类: 移植错（rust 过渡序列状态机缺 FC 冻结段；emu=真相（D3），冻结为真机
  过渡代码不进主循环 `inc $2D` 的真实时序）
- 处理: 回 porter——在 rust 上电初始化与 title→menu 过渡路径复刻 3+3 帧 FC 冻结；
  修复判据=case-fcprobe 逐帧 FC 差恒 0 + case-001 敌槽轨迹双侧同值
- 门禁: python3 tools/l4_diff_run.py run case-fcprobe → 现状差 +6（本文 §2 复现）
```

## 5. 修复落地（2026-10-29 porter 修，本节补注）

**改动面**（commit 见 git log）：
- `state.rs`：FrameMemory 增 `title_no_return` 旗；
- `mainloop.rs main_loop_iter`：FC 自增改门控（`!title_no_return`）——asm `inc FrameCount`
  （prg7:207）在 MainLoop **迭代尾**，例程不返回的帧不自增；
- `title_bridge.rs`：旗先于 spin 消费置位（内旋族相位 $01/$15/$1B——spin tick=例程内部
  等待帧同冻；0x00 工作段同拍推进相位故其 spin tick 入相位已属族）+head_hit 帧置旗
  （Start 命中帧=转换例程体同帧执行，emu f310 冻实录）；
- `title/mod.rs`：$01 臂与 $15|$1B 臂**融合尾段**（相位推进并入最后工作段；去段间
  SpinWait——asm 段尾内旋等待不返回主循环，段间无独立帧）；顺正占位组 unported 表址
  （0x12→0x82ED/0x17→0x911A，旧形误记 0x939E）。

**判据结果**：
- ✅ 判据一（fcprobe 逐帧 FC 差恒 0）：达成——452 可比帧 **FC 0 分歧**；相位仅剩
  1 处单行差（f7：0x02 fade_delay 臂时长差 1 帧，f8 起自愈）——遗留登记。
- ❌ 判据二（case-001 敌槽轨迹双侧同值）：**未达成**——FC 对齐后爬行残差形态改变
  （f810 起 rust 190/190/190/191 vs emu 196/200/200/200，f950 起路径分叉扩大）：
  爬行**转向/撞角族**（CrawlerInsideCornerCheck $9AA8/OutsideCornerCheck $9AC6/
  四例行）存在 FC 之外的独立分歧面——新考证任务（本档不修，路由 porter 考证轮）。
- 门禁：gates check PASS=13 FAIL=0；**L4 76 格 match 集合零缩水**（FC 对齐未引起
  任何锚定点回退——两窗均在 title 域、L4 采集点不采样该窗）；cargo test 457 过
  0 败（含新钉 `fc_frozen_on_no_return_title_arms`）。

**假设区（未证）**：$1C GameOver 臂（3 spin）同族疑同冻（本窗无证据未动）；0x02
fade_delay 臂 1 帧时长差（f7 单行、f8 自愈）疑 glue 延迟计数边界——两案留考证。

## 6. 证据指针

- `analysis/traces/case-fcprobe.frames.jsonl`（emu 460 帧逐帧 FC，watch 002D）
- `shots_rust/case-fcprobe.state.jsonl`（rust 452 帧逐帧 zp；f352-358 采样缝=过渡窗自身）
- case-001 六采样点 ram.hex（95/9C 等，capture 面 +7 实录）
- prg7_engine.asm:11512（ShouldCrawlerMove）/ :8740 区（GetEnemyData 槽偏移语义）/
  enemies/crawler.asm（6/8 门调用方）
- mainloop.rs:866（rust 门同构）；ram_map.rs:34（FC=$2D 语义锚）
- BUG-089（bug-ledger.md:60，渲染/采样时序族——与本残差独立，勿混）
