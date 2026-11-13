# 核心状态语义对拍（`.sem`）首发实测

> 2026-11-13 · 用户令（m13355）「学习 super-metroid-decomp 的 sem 对拍机制，
> 建立核心状态的快速对拍机制」·
> 工具 `tools/sem.py`，规格 `docs/spec/core-state-machine.md` §11·
> Evidence **A**（两侧同一用例、同一帧号逐列比对，数值可复现）Confidence 92

## 1. 为什么需要它（与 L4 是两套栈，不合并）

| | L4 像素对拍 | 本工具（`.sem` 语义对拍） |
|---|---|---|
| 判据 | 特定帧整屏像素 0 差 | 每帧 15 个状态列的数值 |
| 分辨率 | 稀疏关键帧（112 格） | **每帧一行**（2600 帧） |
| 失败语义 | 「这帧画错了」 | 「这一帧的状态机/计时/坐标不对」 |
| 依赖 | 需要 PPU 渲染腿 | **不需要 PPU**（不跑 capture） |
| 成本 | 440s（run-all 矩阵） | oracle 55s + rust 60s + diff **毫秒级** |

`~/workspace/super-metroid-decomp/docs/plan/two-verification-stacks.md` 已定
「两栈不合并」，理由三条（分辨率/失败语义/依赖）对本案同样成立。
本工具补的是**中间层**：像素栈告诉你「画错了」，语义栈告诉你「哪个变量先错、错在第几帧」。

## 2. 列契约（16 列，全部在 ZP/IRAM，规格 §11）

| 列 | 地址 | 语义 | | 列 | 地址 | 语义 |
|---|---|---|---|---|---|---|
| `fc` | `$2D` | FrameCount | | `ia` | `$74` | InArea |
| `gm` | `$1D` | GameMode | | `rm` | `$5A` | RoomNumber |
| `mr` | `$1E` | MainRoutine | | `st` | `$0300` | Samus.status |
| `tr` | `$1F` | TitleRoutine | | `sy` | `$030D` | Samus.y |
| `gp` | `$31` | GamePaused | | `sx` | `$030E` | Samus.x |
| `nr` | `$20` | NextRoutine | | `hp` | `$0106:$0107` | Health（BCD） |
| `t3` | `$2C` | Timer3 | | `nm` | `$1A` | NMIStatus（**相位敏感，默认排除**） |
| `cb` | `$23` | CurrentBank | | `bsp` | `$24` | BankSwitchPending |

`nm` 被标为相位敏感的原因（实测）：`$1A` 是 NMI 与主循环之间的握手瞬时位，
两侧采样点落在握手窗口的不同侧 ⇒ oracle 2593/2600 帧读 00、rust 2582/2583 帧读 01。
它**不携带行为信息**，默认不参与比较但显式打印在 `phase_sensitive=`，需要时 `--only nm`。

## 3. 纪律（三条，代码里是硬约束不是注释）

1. **不写 0 冒充**：某侧取不到的列记 `<na>` 并计入 `na_cols` 打印（AGENTS §3）。
2. **只比双方都有值的列**：单侧缺失是「未接线」不是「不一致」，由 `not_covered` 显式列出。
3. **按帧号对齐，不按行序**：Rust 侧只在 spin 出口落行（掉帧窗/长例程忙跨帧无行），
   oracle 每帧一行。按行序对齐会把「未采样」误报成「不一致」——首轮实测即踩到
   （oracle f2 vs rust f3），已改按帧号并输出 `FRAMES_ONLY_ORACLE/RUST`。

## 4. 首发实测（case-password-justin，2600 帧）

命令：`python3 tools/sem.py oracle-case case-password-justin` →
`python3 tools/scenario_run.py run analysis/sem/case-password-justin.sem.txt` →
`python3 tools/sem.py case case-password-justin`

- oracle：2600 帧 / wall 54.9s / captures 0（三区间面 `zp` 512hex + `samus` 32 + `health` 4）
- rust：**2583 状态行**（17 帧为 span 无缝——列表 `2,4,5,6,301,302,303,502,503,504,1191..1197`）
- 共同帧 2583；**完全一致 1178 帧**

逐列分歧帧数：

| 列 | 分歧帧 | 判读 |
|---|---|---|
| `fc` | 1405 | **真差：恒定 +2 相位**（见 §5） |
| `t3` | 353 | 派生（Timer3 每 10 帧递减，相位差导致切换帧错位） |
| `st` | 41 | 集中在 f1190-1199 转场邻域 |
| `mr` | 10 | 同上 |
| `sy`/`sx` | 各 2 | 同上 |
| `bsp`/`ia`/`hp` | 各 1 | 同上 |

`fc` 差值分布（rust − oracle）：`0` × 1178、`+1` × 1（f1189）、`+2` × 1392、
`−254` × 12（u8 回绕，等价于 +2）。

**结论：f1189 之前 1178 帧语义完全一致；从 f1189（密码提交帧）起 rust 恒定领先 2 帧。**

## 5. 这与 L4 残余是同一件事

L4 始终残留的两格 `f700=23px / f1100=42px`，此前已证：
emu 的 f700 截图与 **rust f698** 逐像素相同（f1100 与 rust f1098 相同），
且光标闪烁由 `FrameCount & $08` 驱动（`prg0_title.asm:2568-2612`）
⇒ 残差 = 同一画面在 ±2 帧处取景。

本工具用完全独立的判据（状态列数值，不碰像素）复现了同一个 +2 相位差，
并**把它定位到唯一的引入点 f1189**——这是像素栈做不到的（像素栈只知道「画错了」）。

## 6. 复现

```bash
python3 tools/sem.py watch-lines                                  # 看 oracle 需要哪 3 条区间
python3 tools/sem.py oracle-case case-password-justin             # 生成 oracle 用例
python3 tools/scenario_run.py run analysis/sem/case-password-justin.sem.txt
python3 tools/sem.py case case-password-justin --meta             # 端到端 + 指纹
```

产物在 `analysis/sem/`（gitignored）。**`.sem` 内容不入库**，
入库的是 ①用例 DSL（输入真相）②`<name>.sem.meta.json` 指纹（含 `note`：行为合法迁移时必须写明为什么变）。

## 7. 遗留

- **未修**：`fc` +2 相位（引入点 f1189，即 START 消费 → CheckPassword 链 → InitializeGame 链）。
  下一步终审对象；修好后本 diff 预计收敛到转场邻域以外全 0，同时应关掉 L4 最后两格。
- 只跑了 1 个用例。其余 13 个 L4 用例可同法接入（每个 +55s oracle 一次性成本，之后 diff 毫秒级）。
- `wram`/`pw` 两面尚未纳入列契约（密码字节 `$6988-$6999` 值得单独一张 `.sem`）。
- `--meta` 的「≥3 连跑 sha256 全同」金面前置尚未落成脚本（当前靠人工）。
