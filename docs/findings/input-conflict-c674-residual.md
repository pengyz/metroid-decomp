# input-conflict c674 的 84px 残差：参考图取点伪差（定界）

> 结论（A 级）：`case-mechanism-input-conflict` 采集点 c674 的 84px 不是实现差，而是**参考 png 与它同一次采集的 PPU dump 不是同一时点**造成的取点伪差。
> 决定性证据：参考 png c674 与 c676 **逐字节相同**（sha `472b8dd8ce84…`），两帧的 emu PPU dump 也**逐字节相同**——但在该 dump 的旧内容（NT 第 31 列还是 `09/4c/50`）下画不出这张 png；而该 png 与「第 31 列已写成 `05`」的 rust c676 截图**逐像素相同（0px）**。
> 修法在采集侧（png 与 dump 同点取，或比图统一用同代 dump 渲染）。本文件不改 `shots/`、判据、白名单、阈值；也不改代码。

## 0. 一句话与边界

- 本文件只定界，不修。`crates/`、`gates/`、`tools/`、`shots/` 零改动。
- 与 #34/#35 的关系：#34/#35 两张卡已由 Human 取消，本文件是同一结论的落地稿，取证未重做，只在成稿时用命令把数字重新量了一遍（下文命令均可直接跑）。
- 参考图代际：本文所有 sha 取自当前 `shots/` 与 `analysis/traces/` 的同一次采集（`analysis/traces/case-mechanism-input-conflict.frames.jsonl` 与 `shots/case-mechanism-input-conflict_f0067*.png` 同为 2026-11-23 那批）。换一次重采后 sha 会变，**结论的判据是「png 与同次 dump 不同时点」这个矛盾形式，不是具体 sha 值**。

## 1. 症状：84px，两簇，8 行高

参考 `shots/case-mechanism-input-conflict_f00674_c674.png` vs rust `shots_rust/case-mechanism-input-conflict_f00674_c674.png`：

- 差异 84 px，bbox `(100,81)-(132,88)`；行固定 `y=81..88`（8 行）。
- 两簇：`x=100..111`（覆盖 12 列、实占 11 列，104 为空）、`x=123..132`（覆盖 10 列、实占 8 列，127/128 为空）。
  > 【2026-11-25 更正】初稿把「覆盖范围」写成「列数」；按逐列计数实占 11/8 列（对影复核对上）。不影响 84px 与形态判断。
- 逐列像素数（命令见 §6.2）呈「字符笔画」形态：每列 3–7 px，不是整块矩形 ⇒ 不是精灵整格位移，也不是调色板整屏偏色。

同案的其它层：OAM `$0200-$02FF` 在 c674 两侧**逐槽 0 差**（§3 表），chr/palette 两侧 0 差，因此剩下的只能是 NT 第 31 列那格背景。

## 2. 决定性矛盾：png 与同次 dump 不是同一时点

三条独立事实（每条都能单独复现）：

1. **参考图两帧同 sha**：`shots/…_f00674_c674.png` 与 `shots/…_f00676_c676.png` 都是 `472b8dd8ce846cb75162621d6909cb206218938cebb2cad02ad8e3acc287ed95`。
2. **emu dump 两帧逐字节相同**：`analysis/traces/…_f00674_c674.ppu.hex` vs `…_f00676_c676.ppu.hex` → 0 字节差（12323 B 全长）。
3. **该 dump 的内容画不出那张 png**：emu dump c674 与 rust dump c674 只差 **1 字节**（尾 3 字节里最后一个，属 dump 尾部元信息），即两边 PPU 内容当时是同一态；而 rust c674 的截图与参考 png 差 84px。反过来，参考 png 与 **rust c676** 截图 0px，而 rust c676 的 dump 相对 c674 已多写 56 字节（NT 第 31 列）。

也就是说：参考 png 记录的是「第 31 列已流送完」的画面，但它配套的 dump 记录的是「流送前」的 VRAM。rust 侧 png 与 dump 同代（c674 都是旧态、c676 都是新态），所以差 84px 的只有「参考 png 对 rust c674」这一对。

## 3. 交叉对照表（全部实测）

| 项 | 参考 c674 | rust c674 | rust c676 | 说明 |
|---|---|---|---|---|
| png sha（前 8） | `472b8dd8` | `2e4e5872` | `92eacc16` | 参考 c674 与参考 c676 **同 sha** |
| png 像素差（对参考 c674） | 0 | **84** | **0** | 参考图内容 = rust c676 内容 |
| PPU dump 差（对 emu c674） | 0 | **1 字节**（尾） | 57 字节 | rust c674 与 emu c674 内容同态 |
| NT 第 31 列（32 行） | `01/03/09/4c/50/ff` 混 | 同 emu c674 | 出现 `05` | 差的是 `09/4c/50 → 05` 的行，共 28 行 |
| OAM `$0200-$02FF` | — | **0 槽差**（sha `c6ea81e2`） | 23 槽差（屏外槽） | c674 精灵面完全一致 |
| chr / palette | — | 0 / 0 | 0 / 0 | 非图形表差 |

补充两条用于排除「精灵层」「调色板」的干扰：

- c676 的 OAM 有 23 槽差但像素 **0px** ⇒ 那些槽都在屏幕外（y≥0xF0 或 x=240 出屏），不产生可见像素；这与「c674 精灵面 0 槽差」一起说明 84px 不来自精灵。
- 差异只落在 NT0/NT2 的第 31 列（`nt1/nt3` 0 差），是 MMC1 竖直镜像的正常表现（同一列在两块表各出现一次）。

## 4. 为什么只剩 c674（c673/c676/c678 的对照）

- **c673**：已由 #11 归零（`nmi-entry` 取像点修复），当前 0px；它的参考 png sha 是 `48bb94b9…`，与 c674/c676 的 `472b8dd8…` 不同，属另一代。
- **c676**：参考 png 与 rust c676 都是「新态」，0px。它的参考 png 与 c674 **同 sha**，正说明 c674/c676 的参考 png 是同一张新态图（这就是 §2 的矛盾来源）。
- **c678**：两侧 dump 第 31 列都是 `05`（都已流送完），像素 0px。也就是说「新态对新态」时两侧一致，只有 c674 的「参考 png 新态 vs rust 旧态」这一对不齐。

## 5. 采集侧修法建议（本轮不做）

1. **首选**：采集脚本在同一时刻同时抓 png 与 PPU/RAM dump（现在的 emu 路径在帧界先 dump、后截屏，两者跨了一次列流送）。落点：`tools/scenario_capture.lua` 的采集回调里把 `capture_ppu` 与 `saveScreenshot` 放在同一个 spin-exit 缝、同一 `frame` 值下。
2. **次选（比对侧）**：比图时统一用「同一代 dump 渲染出的图」或统一用「实际截屏」，不要一边用 png、一边用 dump 推。
3. **不建议**：把 c674 记为已知偏差后改判据/白名单。当前 84px 已有可解释的采集侧原因，改判据会把这个解释一起埋掉。

落地这张修法需要 `tools/` 写面，请 PMO 另开执行卡；本卡不领修复。

## 6. 复现命令（≥3 行，均可直接跑）

```bash
# 6.1 参考图 c674 与 c676 同 sha（A 级）
sha256sum shots/case-mechanism-input-conflict_f00674_c674.png shots/case-mechanism-input-conflict_f00676_c676.png

# 6.2 84px、bbox 与两簇（A 级）
python3 - <<'PY'
from PIL import Image
ref=Image.open('shots/case-mechanism-input-conflict_f00674_c674.png').convert('RGB')
for frame,tag in ((674,'c674'),(676,'c676')):
    p=f'shots_rust/case-mechanism-input-conflict_f{frame:05d}_{tag}.png'
    im=Image.open(p).convert('RGB')
    pts=[(i%256,i//256) for i,(a,b) in enumerate(zip(ref.getdata(),im.getdata())) if a!=b]
    xs=[x for x,_ in pts]; ys=[y for _,y in pts]
    print(tag,'diff',len(pts),'bbox',(min(xs),min(ys),max(xs),max(ys)) if pts else None)
PY

# 6.3 emu c674 与 c676 的 dump 逐字节相同；与 rust c674 只差 1 字节；rust c676 多 56 字节（A 级）
python3 - <<'PY'
def load(p):
    return bytes.fromhex(''.join(open(p).read().split()))
E4=load('analysis/traces/case-mechanism-input-conflict_f00674_c674.ppu.hex')
E6=load('analysis/traces/case-mechanism-input-conflict_f00676_c676.ppu.hex')
R4=load('shots_rust/case-mechanism-input-conflict_f00674_c674.ppu.hex')
R6=load('shots_rust/case-mechanism-input-conflict_f00676_c676.ppu.hex')
d=lambda a,b: sum(x!=y for x,y in zip(a,b))
print('emu674 vs emu676', d(E4,E6), 'emu674 vs rust674', d(E4,R4), 'rust674 vs rust676', d(R4,R6))
print('NT0 col31 (emu674):', [E4[0x2000+31+32*r] for r in range(8)],
      '| NT2 col31 (rust676):', [R6[0x2800+31+32*r] for r in range(8)])
PY

# 6.4 OAM 精灵面在 c674 两侧 0 槽差（A 级）
python3 - <<'PY'
import hashlib
def load(p): return bytes.fromhex(''.join(open(p).read().split()))
e=load('analysis/traces/case-mechanism-input-conflict_f00674_c674.ram.hex')[0x200:0x300]
r=load('shots_rust/case-mechanism-input-conflict_f00674_c674.ram.hex')[0x200:0x300]
print('oam sha emu', hashlib.sha256(e).hexdigest()[:12], 'rust', hashlib.sha256(r).hexdigest()[:12],
      'slots diff', sum(1 for i in range(64) if e[4*i:4*i+4]!=r[4*i:4*i+4]))
PY
```

## 7. 证据级别与指针

| # | 主张 | 级别 | 指针 |
|---|---|---|---|
| 1 | c674 差异 84px、bbox `(100,81)-(132,88)`、两簇 | A | §6.2 命令输出 |
| 2 | 参考 c674/c676 png 同 sha `472b8dd8…` | A | §6.1 |
| 3 | emu dump c674/c676 逐字节相同；emu c674 与 rust c674 只差 1 字节 | A | §6.3 |
| 4 | 参考 png 与 rust c676 0px；与 rust c674 84px | A | §6.2 |
| 5 | NT 第 31 列 `09/4c/50 → 05`、nt1/nt3 0 差 | A | §6.3（列采样）+ dump 全量比 |
| 6 | c674 精灵面 OAM 0 槽差 | A | §6.4 |
| 7 | 成因是「png 与 dump 取点不同时点」 | **B** | 由 2–5 推定：dump 内容无法解释同次 png，而 rust 侧同代自洽；采集脚本的具体取点顺序未逐行审计（见 §8） |

## 8. 未覆盖 / 下一步

- **未做**：逐行审计 `tools/scenario_capture.lua` 里 `capture_ppu` 与截屏的调用顺序（属 `tools/` 写面，本卡边界外）。这条是把第 7 条从 B 升到 A 的唯一缺口。
- **未做**：重采验证（重采会改 `shots/` 代际，本卡明确不动参考图）。修法落地后，期望的验收是 c674=0 且 c673/c676/c678 不变（同 run 三采集点）。
- **C 级假设（不进结论）**：emu 侧的 png 可能来自「下一帧的画面已渲染但 dump 尚未重抓」的窗口；这一点没有 trace 证据，仅作为解释候选。
