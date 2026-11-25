# #16 之后的两处精灵族残差：定界（Tourian10 c1731 / Ridley37 c1701·c1709·c1729）

> 范围：#16（Metroid 负速钳位字节形 + dessgeega `data1F` 动画门）落地后，`run_room` 两房仍有非零格。
> 本文只做**定界**（哪一格、哪个槽、哪个字段、哪一侧、能否归到单一根因），不改代码、不改判据、不重采金标。
> 证据分级按 AGENTS.md §3：A=字节/像素/日志实测，B=代码路径+数据流推断，C=假设（只在 §6）。

## 0. 结论摘要

| 房 | 残余格（实测） | 首分歧槽 | 实测差 | 本文定界 |
|---|---|---|---|---|
| Tourian10 | `c=1731 g=1740 phase=live diff=291` | 槽 9（级联 9..17） | seam 相位 **0 槽差 / 0px**；live 相位 **9 槽差 / 291px** | **非对象逻辑差**：同一编号的状态字节两腿逐字节相同（`rust f1731 ≡ emu g1740`，全槽 0 差），差只出现在 live 精灵页 ⇒ **精灵页构建相位**（哪一 tick 的状态进入本帧显示）。归「判等相位」族，与 #11 同族但成因不同（#11 是取像点晚一帧，这里是重复帧上 `choose_phase` 选了 live）。 |
| Ridley37 | `c=1729 g=1737 phase=live diff=26216` | 无（`oam_slots=0`） | live 26216px、对**任一** emu 帧最小 12990px；seam 相位对 emu f1736/f1737 = **981px** | **非敌人层**：live OAM 与 emu 页逐槽**完全相同**、CHR/名表/调色板 dump 全 0，但 live 画面在 emu 任一帧都找不到对应 ⇒ live 渲染路径在该换房/滚动瞬态帧画出了未显示过的画面；同帧 seam 路径 981px 才是同画面差。 |
| Ridley37 | `c=1701 g=1703 phase=live diff=68` | 槽 30（级联 31、35–39） | live 68px（seam 1093px ⇒ live 才是正确相位） | **状态层已排除**：`rust f1701 ≡ emu g1702 ≡ g1703`（Ens `$0400-$04FF` + EnsExtra `$6AF4..` 全 14 槽 **0 字节差**）。差在**绘制/OAM 生成**：同一对象出 `tile=$A0` vs `$A2`（同 x/y/attr），且 rust 侧多出非停泊槽（emu 停泊）。未定界到单一根因，见 §4 下一步。 |
| Ridley37 | `c=1709 g=1713 phase=live diff=48` | 槽 30（级联 31） | live 48px（seam 1133px） | 同 c1701：`rust f1709 ≡ emu g1713` 状态 0 差；OAM 槽 30/31 tile `$A0` vs `$A2`。同一对象、同一 tick 的**帧表落点**差 2 字节。 |

一句话：**Tourian10 的 291px 与 Ridley37 的 26216px 都不是敌人逻辑差**（一个是重复帧相位选择，一个是换房瞬态下 live 渲染路径）；**Ridley37 的 68/48px 是真差，但已排除状态推进**，差在 `DrawEnemy` 的帧表索引/OAM 停泊位生成。

## 1. 复现命令（±0）

以下命令在仓库根执行（本机为 `/home/peng/workspace/metroid-decomp`），只用既有 emu 采集与 rust 产物，不启动 Mesen2。

### 1.1 两房复扫

```bash
python3 analysis/scratch-sprite-residual/run_room.py --area 3 --room 10 \
  --reuse-emu-old case-oos2-t10 --analyze-only
python3 analysis/scratch-sprite-residual/run_room.py --area 4 --room 37 \
  --reuse-emu-old case-oos2-r37 --analyze-only
```

期望输出（本文实测，逐字）：

```
case-sprite-t10: frames=81 ok=70 nonzero=1 errors=10 max=291
  c=1731 g=1740 phase=live diff=291 oam_slots=9 first=9
case-sprite-r37: frames=81 ok=70 nonzero=3 errors=8 max=26216
  c=1701 g=1703 phase=live diff=68 oam_slots=7 first=30
  c=1709 g=1713 phase=live diff=48 oam_slots=2 first=30
  c=1729 g=1737 phase=live diff=26216 oam_slots=0
```

> `errors=10 / 8` 是采集窗口两端 emu PNG 缺帧（`missing_png`），与残差无关。

### 1.2 Tourian10 c1731：seam 相位 0px / live 相位 291px（A 级）

```bash
python3 - <<'EOF'
from PIL import Image; import glob
d='analysis/scratch-sprite-residual/case-sprite-t10'
def im(p): return Image.open(p).convert('RGB')
emu={g:im(glob.glob(f'{d}/emu/shots/*_f{g:05d}_*.png')[0]) for g in (1738,1739,1740)}
live=im(glob.glob(f'{d}/live/*f01731*.png')[0]); seam=im(glob.glob(f'{d}/seam/*f01731*.png')[0])
def diff(a,b): return sum(1 for p,q in zip(a.getdata(),b.getdata()) if p!=q)
print('seam@c1731 vs emu:',{g:diff(seam,e) for g,e in emu.items()})
print('live@c1731 vs emu:',{g:diff(live,e) for g,e in emu.items()})
EOF
```

实测：`seam@c1731 vs emu: {1738: 0, 1739: 0, 1740: 0}`；`live@c1731 vs emu: {1738: 291, 1739: 291, 1740: 291}`。

### 1.3 同一编号的状态字节两腿相等（A 级）

```bash
python3 - <<'EOF'
import json, glob
rows={json.loads(l)['f']:json.loads(l) for l in open('analysis/scratch-sprite-residual/case-sprite-t10/case-sprite-t10.state.jsonl')}
for f,g in [(1730,1739),(1731,1740)]:
    r=rows[f]; ir=bytes.fromhex(r['w']['iram']); wr=bytes.fromhex(r['w']['wram'])
    e=bytes.fromhex(open(glob.glob(f'analysis/scratch-sprite-residual/case-sprite-t10/emu/*_f{g:05d}_*.ram.hex')[0]).read().strip())
    ens=sum(1 for i in range(256) if ir[0x300+i]!=e[0x400+i])
    ext=sum(1 for i in range(224) if wr[0xAF4+i]!=e[2048+i])
    print(f'rust f{f} vs emu g{g}: Ens 差字节={ens} EnsExtra 差字节={ext}')
EOF
```

实测：`rust f1730 vs emu g1739: Ens 差字节=0 EnsExtra 差字节=0`、`rust f1731 vs emu g1740: Ens 差字节=0 EnsExtra 差字节=0`。
（对齐口径：emu 采点 g 的 RAM 页 = rust `state.jsonl` 的 `f = g-1`；两行都 0 差可自证该口径。）

### 1.4 Tourian10 槽级（A 级）

```bash
python3 - <<'EOF'
import glob,re
def oam(path,f):
    out=[(0,0xF0,0,0)]*64          # 与 run_room.parse_rust_oam 同构：未打印槽=停泊
    for line in open(path):
        m=re.match(rf'\[oam\] f{f} (\d+) y=(\d+) tile=([0-9A-F]+) attr=([0-9A-F]+) x=(\d+)',line)
        if m: out[int(m.group(1))]=(int(m.group(5)),int(m.group(2)),int(m.group(3),16),int(m.group(4),16))
    return out
pg=bytes.fromhex(open(glob.glob('analysis/scratch-sprite-residual/case-sprite-t10/emu/*_f01739_*.ram.hex')[0]).read().strip())[0x200:0x300]
emu=[(lambda y,t,a,x:(x,y,t,a))(*pg[4*k:4*k+4]) for k in range(64)]
for tag in ('seam','live'):
    r=oam(f'analysis/scratch-sprite-residual/case-sprite-t10/rust.{tag}.oam.txt',1731)
    dif=[k for k in range(64) if emu[k]!=r[k] and not (emu[k][1]>=0xF0 and r[k][1]>=0xF0)]
    print(tag,'差槽',len(dif),dif[:12], '槽9:',emu[9],r[9])
EOF
```

实测：`seam 差槽 0 [] 槽9: (83, 104, 195, 35)`；`live 差槽 9 [9,10,11,12,13,14,15,16,17] 槽9: (83, 104, 195, 35)` 而 live 侧槽 9 为 `(81,106,195,34)` —— 整组 9 槽统一 `x−2、y+2、attr` 低位置 0。

### 1.5 Ridley37 c1729 / c1701 的相位与像素（A 级）

```bash
python3 - <<'EOF'
from PIL import Image; import glob
d='analysis/scratch-sprite-residual/case-sprite-r37'
def im(p): return Image.open(p).convert('RGB')
def diff(a,b): return sum(1 for p,q in zip(a.getdata(),b.getdata()) if p!=q)
live=im(glob.glob(f'{d}/live/*f01729*.png')[0]); seam=im(glob.glob(f'{d}/seam/*f01729*.png')[0])
m={g:im(glob.glob(f'{d}/emu/shots/*_f{g:05d}_*.png')[0]) for g in range(1720,1741)}
print('live@1729 vs emu:',min(((diff(live,e),g) for g,e in m.items())))
print('seam@1729 vs emu f1736/1737:',diff(seam,m[1736]),diff(seam,m[1737]))
EOF
```

实测：`live@1729 vs emu: (12990, 1738)`（即 live 与**全部** emu 帧最佳仍 12990），`seam@1729 vs emu f1736/1737: 981 981`；而 live 对 f1737 为 26216。c1701：live 对 f1702/f1703 = 68/68，seam = 1093；c1709：live 对 f1712/f1713 = 48/48，seam = 1133。

### 1.6 Ridley37 状态层 0 差 + OAM 槽 30/31（A 级）

```bash
python3 - <<'EOF'
import json,glob
rows={json.loads(l)['f']:json.loads(l) for l in open('analysis/scratch-sprite-residual/case-sprite-r37/case-sprite-r37.state.jsonl')}
for f,g in [(1701,1702),(1709,1713)]:
    r=rows[f]; ir=bytes.fromhex(r['w']['iram']); wr=bytes.fromhex(r['w']['wram'])
    e=bytes.fromhex(open(glob.glob(f'analysis/scratch-sprite-residual/case-sprite-r37/emu/*_f{g:05d}_*.ram.hex')[0]).read().strip())
    print(f,'Ens差',sum(1 for i in range(256) if ir[0x300+i]!=e[0x400+i]),'EnsExtra差',sum(1 for i in range(224) if wr[0xAF4+i]!=e[2048+i]))
EOF
```

实测：`1701 Ens差 0 EnsExtra差 0`、`1709 Ens差 0 EnsExtra差 0`。

OAM 槽（emu 页取 `_f01702` 的 `$0200-$02FF`，rust 取 `live` f1701；字节序 y,tile,attr,x → 展示为 (x,y,tile,attr)）：

| 槽 | emu | rust | 判读 |
|---|---|---|---|
| 30 | (48,33,**$A0**,$01) | (48,33,**$A2**,$01) | 同 x/y/attr，帧表落点差 2 字节 |
| 31 | (56,33,**$A0**,$01) | (56,33,**$A2**,$01) | 同上 |
| 32/33 | (40,43,$FF,$01)/(48,43,$FF,$01) | 同 | 两侧一致（$FF=空白 sprite） |
| 34 | (56,43,$FF,$01) | 同 | 一致 |
| 35/36 | (24,43,$5E,$00)/(32,43,$5F,$00) | $FF/$FF | emu 有图案，rust 空白 |
| 37 | (240,33,$76,$F0) | (24,33,$76,$01) | emu x=240 出屏（视作停泊），rust 可见 x=24 |
| 38/39 | (240,244,$F0,$F0) 停泊 | (32,33,$7F,$01)/(40,33,$3A,$00) | rust 多出两条可见条目 |

`run_room` 只把「两侧 y 均 ≥0xF0」当停泊 ⇒ 上表 35–39 全部计入差（共 7 槽，`oam_slots=7`）。

上表的复跑命令：

```bash
python3 - <<'EOF'
import glob,re
def oam(path,f):
    out=[(0,0xF0,0,0)]*64
    for line in open(path):
        m=re.match(rf'\[oam\] f{f} (\d+) y=(\d+) tile=([0-9A-F]+) attr=([0-9A-F]+) x=(\d+)',line)
        if m: out[int(m.group(1))]=(int(m.group(5)),int(m.group(2)),int(m.group(3),16),int(m.group(4),16))
    return out
pg=bytes.fromhex(open(glob.glob('analysis/scratch-sprite-residual/case-sprite-r37/emu/*_f01702_*.ram.hex')[0]).read().strip())[0x200:0x300]
emu=[(lambda y,t,a,x:(x,y,t,a))(*pg[4*k:4*k+4]) for k in range(64)]
ro=oam('analysis/scratch-sprite-residual/case-sprite-r37/rust.live.oam.txt',1701)
for k in range(30,40):
    print(k,emu[k],ro[k],'' if emu[k]==ro[k] else '<--')
EOF
```

## 2. Tourian10 c1731 定界

### 2.1 测到的事实（A 级）

1. `oam.json` 该行 `phase=live diff=291 oam_slots=9 first=9`；emu 槽 9 = `(x83,y104,tile$C3,attr$23)`，rust live 槽 9 = `(81,106,$C3,$22)`，级联 9..17。
2. 同一编号状态字节：`rust f1731 ≡ emu g1740`（Ens 256B + EnsExtra 224B 全 0 差，含 `y=119,x=93,animFrame=4,animIndex=7,type=0`）。
3. 像素：`rust seam@c1731 ≡ emu f1738/f1739/f1740`（各 0px）；`rust live@c1731` 对同三帧各 291px。
4. 槽级：`rust.seam.oam.txt f1731` 对 emu 页 `_f01739` **0 槽差**；`rust.live` 9 槽差。
5. emu 三帧图同图：`emu f1739 == f1740`（逐像素），这也是 `choose_phase` 选 live 的直接原因。

### 2.2 定界

- **不是对象位置/动画逻辑差**：同一编号的 `Ens`/`EnsExtra` 全槽逐字节相等（事实 2），动画字段 `animFrame=4 / animIndex=7` 两侧相同。
- **是精灵页构建相位**：seam 路径（上一 tick 克隆）与 emu 显示逐槽、逐像素相同（事实 3、4）；live 路径（本 tick 活体）比 emu 显示多走了一个 tick 的 y/x（槽 9 组 `x−2,y+2` 恰好是该对象 `x95→93`、`y117→119` 的位移方向）。
- **为什么会被记成残差**：`tools/movie_ref/fc_align.py:126-130` 的 `choose_phase` = `"live" if is_duplicate_second(g) else "seam"`，而 `is_duplicate_second`（`:93`）判的是 **emu png(g-1) == emu png(g)**。本格 g=1740 与 g=1739 像素完全重复 ⇒ 规则取 live；但重复帧上 emu 的显示内容等价于「上一 tick 的精灵页」，rust 的 seam 才是同一内容。
- 同一现象也解释了本房的 `c=1729 多候选 cands=[1737,1738]`（配对候选也是重复帧）。

### 2.3 归类与建议

- 族：**判等相位族**（不是敌人逻辑族）。与 #11 的 late-seam 同属「同一编号不同内容来源」，但成因相反：#11 是 emu 取像晚一帧，这里是 rust 侧 live/seam 选错。
- 建议（另开卡，写面在 `tools/movie_ref/` 或 `tools/l4_diff_run.py`，不在本卡范围）：重复帧上允许「seam 或 live 取较小者」并记录所选相位；或把 `is_duplicate_second` 的重复判定从「图像重复」改为「状态重复」。**不得直接改阈值/白名单。**
- 若不改判据，则本格在报告中应标注为「相位伪差（seam 0px）」，不计入敌人层残差。

## 3. Ridley37 c1729 定界

### 3.1 测到的事实（A 级）

1. `oam.json`：`c=1729 g=1737 phase=live diff=26216 oam_slots=0`；`c=1730..1732 diff=0`（单帧瞬态）。
2. OAM：`rust live f1729` 与 emu 页 `_f01736`/`_f01737` 均 **0 槽差**；`rust seam` 对同页 30 槽差（seam 是上一 tick 克隆）。
3. 像素：live 对 emu f1737 = 26216px，对**任一** emu 帧最小 12990（f1738）；seam 对 f1736/f1737 = 981px（两帧字节相同）。
4. PPU dump（emu `_f01737` vs rust seam `_f01729`，len 均 12323）：`chr`、`nt0..nt3`、`palette` 全 0 差；仅尾 3 字节（scrollX/scrollY/ctrl）不同：emu `000000` vs rust `fe0090`。
5. RAM `$FD`（ScrollX）两侧 c1729 均 0（rust f1728=$FE→f1729=$00；emu g1735=$FE→g1736=$00）。

### 3.2 定界

- **不是敌人层**：OAM 逐槽相同（事实 2），PPU 静态面全 0（事实 4）。
- live 的 26216px 不能归给「某对象」：rust live 画面在 emu **所有帧**里最佳仍差 12990（事实 3），说明 live 渲染路径在该瞬态帧画出的整屏内容（背景/滚动）在 emu 上从未显示过 —— 与 `fc_align.py:9` 注释「载房窗内一律取 seam」所指的同类问题（换房窗内 live 不可比），但本帧未被 `room_load` 命中也未进 `load` 集合。
- 同帧 seam 的 981px：bbox rows 47–160、cols 16–177，形态为若干精灵状小块（左边缘竖条 + 三处团块），非整屏平移（全范围循环平移最小仍 11510）⇒ 与 seam 精灵页「上一 tick 克隆」一致（30 槽差却只有 981px，说明差异槽大多落在停泊/出屏位置）。这条线索留在 §6 假设区。

### 3.3 归类与建议

- 族：**换房/滚动瞬态族（渲染路径）**，与 #12（room-load 相位）邻接但未确认同一根因。
- 建议：把 c1729 加入 room-load/换房窗（或让 `choose_phase` 对该案强制 seam），再复算；届时期望 981（而非 26216）。这是判据侧改动，需另开卡。

## 4. Ridley37 c1701 / c1709 定界（真差，未到单一根因）

### 4.1 已排除（A 级）

- **状态推进差**：`rust f1701 ≡ emu g1702 ≡ g1703`、`rust f1709 ≡ emu g1713`，Ens 256B + EnsExtra 224B 全 0 差；对齐窗口扫描（Ens 差字节数）在 `f = g-1` 上唯一取 0。
- **相位选错**：这两格 live 才是正确相位（live 68/48 vs seam 1093/1133，且 live 对 g±1 仍最小）。
- **CHR/名表/调色板**：两格 `ppu` 各层全 0。

### 4.2 测到的差异（A 级）

- 槽 30/31：`tile=$A0`（emu） vs `$A2`（rust），x=48/56、y=33、attr=$01 全同 ⇒ 同一位置、同一属性，**帧表落点差 2 字节**（即相邻两个帧槽）。
- rust 侧多出可见条目（槽 35–39），emu 侧对应槽为空白/停泊（见 §1.6 表）。

### 4.3 机制指针（B 级）

- ROM：`DrawEnemy ; 07:DD8B`（`refs/m1disasm/SRC/prg7_engine.asm:6061`）读 `EnsExtra.0.animFrame,x`；`$FF` 走 `ClearObjectCntrl ; 07:DF2D`（`:6068` 的 `jmp`），否则 `asl/tay` 后按 **bit7 选 `EnFramePtrTable1/2`** 取帧指针（`:6081-6097`）。⇒ 帧表落点由 `animFrame` 的 bit7 与低 7 位共同决定，`$A0`→`$A2` 的差等价于 `animFrame` 相差 1（同表内相邻项）。
- Rust：`crates/game-core/src/render/enemy_sprite.rs:90` 同源取 `mem.ens_face(x).anim_frame()`；`:92` 帧空（`$FF`）时只 `set_obj_cntrl(0)` 返回，是否把剩余 OAM 槽置为停泊值由后续 `draw_sprite_stream`（`crates/game-core/src/render/oam.rs:81-104`，`oy=sy+dy-1`、`ox=sx+dx`）决定。
- 由于两腿 `EnsExtra.animFrame` 字节相同（§4.1），`$A0/$A2` 不可能来自 `animFrame` 数值差 ⇒ 剩余可能：①两侧对**同一 OAM 槽号**归属了不同对象（绘制顺序/槽位复用不同）；②同一对象但 rust 多取了一次帧表偏移（例如把 `hi` 位或 place 数据混入）；③rust 未复刻 ROM 的「空白帧 → 清剩余槽」路径而留下上一帧条目。三者都指向**绘制路径**，不是 `metroid.rs`/`sidehopper.rs` 的敌人状态逻辑。

### 4.4 下一步（未完成，需代码写面）

1. 在 `draw_at` 入口打印 `(x, animFrame, frame_ptr_offset, oam_page_pos)`（临时探针，属 `crates/game-core/src/` 写面，需另开卡）；用同帧 emu 的 `$0200` 页反推每个对象的起始槽，确认槽 30 归属。
2. 对照 ROM `07:DD8B` 的 `ClearObjectCntrl`（`:6097` 之后）与 rust 的空白帧早退分支，核对「剩余槽停泊」是否一致。
3. 只有在①–③中定位到一个之后，才考虑改渲染代码；本卡只交定界。

## 5. 族划分（供 PMO 拆卡）

| 族 | 格 | 根因位置 | 证据级 | 建议卡 |
|---|---|---|---|---|
| 判等相位（重复帧选 live） | Tourian10 c1731（291） | `tools/movie_ref/fc_align.py:93,126-130` | A | 判据卡：重复帧允许 seam/live 取小并记录相位 |
| 换房瞬态 live 渲染 | Ridley37 c1729（26216） | room-load 窗/live 渲染路径 | A 观测量 + B 路径 | 并入 #12 或新开「换房窗相位」卡 |
| 绘制帧表/OAM 停泊 | Ridley37 c1701(68)、c1709(48) | `render/enemy_sprite.rs:60-140`、`render/oam.rs:81-104` | A 现象 + B 指针 | 新开「敌帧表落点与停泊槽」卡（需代码写面） |

## 6. 假设区（C 级，未证实）

1. c1729 的 981px 精灵状团块＝seam 页面「上一 tick 克隆」导致的对象显示晚一 tick（与 c1731 同族机理），但本格 live 背景不可比，无法用像素直接裁决。
2. c1701 的 `$A0/$A2` 可能来自 dessgeega 在 `EnemyFlipAfterDisplacement` 后 `animIndex` 与 `animFrame` 的**不同步**（#16 只修了符号门，未覆盖面切换）；也可能来自 OAM 槽位复用，两者未区分。
3. emu 侧用 `x=240` 表示出屏（而非 `y≥$F0` 停泊）——若成立，`run_room` 的停泊判定口径会低估「rust 多画」类差异；需在另一卡里核对 ROM 的 `ClearObjectCntrl` 究竟写什么值。

## 7. 证据指针

| 结论 | 证据 | 级别 |
|---|---|---|
| 两房复扫数字（70/1/291、70/3/26216） | §1.1 命令输出、`case-sprite-{t10,r37}.oam.json` | A |
| Tourian10 seam 0px / live 291px | §1.2 命令与输出 | A |
| 同编号状态字节相等（两对齐对） | §1.3 命令与输出 | A |
| Tourian10 seam OAM 0 槽 / live 9 槽 | §1.4 命令与输出 | A |
| c1729 live 对全部 emu 帧最佳 12990、seam 981 | §1.5 命令与输出 | A |
| c1701/c1709 状态 0 差 | §1.6 命令与输出 | A |
| 槽 30/31 tile `$A0` vs `$A2`、35–39 停泊差 | §1.6 表 | A |
| `choose_phase` 重复帧取 live | `tools/movie_ref/fc_align.py:93,126-130` | B |
| 帧表由 `animFrame` bit7/低7位选表 | `refs/m1disasm/SRC/prg7_engine.asm:6061-6097`；`crates/game-core/src/render/enemy_sprite.rs:90-92` | B |
| 换房窗 live 不可比 | `tools/movie_ref/fc_align.py:9` 注释 + 本卡 §3 观测 | B |

## 8. 未覆盖

- 未启动 Mesen2（全部用既有 emu 采集 + rust 产物），因此没有新采点；`errors=10/8` 的缺帧未补。
- 未在 rust 侧加任何探针（写面只有 `docs/findings/`），c1701 的对象归属仍靠反推，未用绘制序列直证。
- c1729 的 981px 未逐簇归对象（只给了形态与 bbox）。
- 三格都没有改动判据/白名单/金标；也没有以此为理由升级基线。若后续在判据侧修好相位选择，应重跑 `tools/l4_diff_run.py run-all` 并把新数字写回本文件（原地加日期更正，不删旧数）。
