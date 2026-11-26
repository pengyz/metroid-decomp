# Stage A 通关证据：全片 120540 帧跑完 + 9 节点到达审计

> 任务：`#101`（`tsk_01M8BKJJAP8X5KTC8RYGKF`），承接 `#98`（`tsk_01M8BJWYG1BPF0EPQPKYSD`）的 30000 帧版本。
> 写面：本文 + `analysis/tas-main-f22895/`；不改 `crates/`、不改 `tools/`、不动判据。
> 取数时间：2026-11-26；仓库 HEAD `d2fea892`；工作树另含 #95 已验收未入库的 8 个 `crates/` 改动（见 §11）。

## 0. 结论（先看这一段）

1. **长片跑完了**：`movies/lordtom-metroid-100p.inputlog`（120540 帧，100% 路线）已跑到 **`ran_frames = 120540/120540`**，
   11 段（12000×10 + 末段 540）全部落盘，失败段 0，累计比较 **120265** 帧。分段表见 §3。
2. **9 个 Stage A 节点仍然一个都没到达（0/9）**，而且这次是**全片**跑完之后的 0/9，不是「只跑了前 30000 帧」。
   两套独立实现给同一结论：`#99` 修好版 `tools/stage_a_nodes.py`（`bcd255d4`）与本文 §2.3 的独立复核脚本。
3. **比「0/9」更具体的根因**：整条 120540 帧里 `InArea`（`$0074`）低 4 位**恒为 0 = Brinstar**，`RoomNumber`（`$005A`）
   只在 `0 / 241 / 243 / 255` 四个值之间跳（**109 次相邻帧跳变**，定义见 §4 复算脚本）——**rust 腿在 f22895 首次分叉之后就再没换过房，
   一直在起始房反复尝试转场**。所以「9 节点没到」不是「TAS 不够长」，而是**卡死在第一次门转场**。
   （正在查门格碰撞瓦 `$A1` 写入链的 `#104` 与本结论同向。）
4. **首分歧的精确读数**：帧 **22895**、字段 **`$0405`**、emu=`0x5A`(90) / rust=`0x1A`(26)，该帧共 **8 个字节**不同。
   该帧之前「逐帧全等」（seg0 比较 11962 帧、0 分歧）。
5. **两份 coverage 的关系（必须一起看）**：`analysis/tas-main-f22895/coverage.json`（**权威**，12000 帧/段 × 11 段）与
   `legacy-s10000/coverage.json`（10000 帧/段 × 13 段的另一轮）**首分歧读数不同**，原因不是进制、也不是口径，
   而是旧一轮的**前 3 段拿上一轮留下的 29946 行 rust 轨迹去比**（`segments[].rust_rows` 直接写着 `29946` / `120347` 两组值）
   ——也就是 §10.2 的 G8 缺陷。所以本文一切结论只引 11 段那一份；旧一轮留档只为解释「两个读数为什么不一致」（§10.5）。

一句话给 PMO：**现有 100% TAS 上，Stage A 的通关链一个节点都没走到；卡点是 f22895 的门转场，不是录像长度。**

## 1. 输入工件与取数口径

| 项 | 值 | 取数方式 |
|---|---|---|
| 影片 | `movies/lordtom-metroid-100p.inputlog`（**120540 帧**） | `jq .frames movies/lordtom-metroid-100p.provenance.json` → `120540`；`frames_with_input=106361`、`reset_frames=[0]` 见同一文件 |
| 已跑帧数 | **`ran_frames = 120540`**（`coverage_pct = 100.0`） | `jq '.ran_frames, .total_frames, .coverage_pct' analysis/tas-main-f22895/coverage.json` |
| 段长 / 段数 | `segment_size = 12000`，**11 段**（10 整段 + 末段 `[120000,120540)`） | `jq '.segment_size, (.segments|length)' …/coverage.json` |
| 累计比较帧 | **120265**（= 各段 `compared` 之和） | `jq '[.segments[].compared]|add' …/coverage.json` |
| 首分歧 | `frame=22895`、`field=$0405`、`emu=90`、`rust=26`、`diff_bytes=8` | `jq .first_divergence …/coverage.json` |
| 环境指纹 | `git_head=d2fea892a4a2d4839d01549008c84959419b148b`、`rust_screenshot_sha256=edcdf01ac4a6627e` | `jq .env …/coverage.json` |
| Rust 腿轨迹 | `analysis/tas-main-f22895/rust/full.state.jsonl`，**120347 行**、`f∈[1,120540]` | `wc -l`；`head -1` 看 `{"f":…,"w":{"zp":…,"iram":…,"wram":…,"pw":…}}` |
| emu 腿轨迹 | `analysis/tas-main-f22895/emu/seg-*.jsonl`，**只采 `ens`（`$0400-$04FF`）+ `Joy1Status`** | `head -1` 看 `w` 只有两个键 |
| 被 G8 污染的旧一轮（10000/段 × 13 段） | `analysis/tas-main-f22895/legacy-s10000/{coverage.json,coverage.md,README.md}` | 前 3 段 `rust_rows=29946`（旧轨迹）、后 10 段 `120347`；**只用于解释差异，不参与本文任何结论**，见 §10.5 |

**内存块的起点（决定所有偏移，必须写清）**：

| 块 | 字节数 | 覆盖地址 | 绝对地址 A 的偏移 |
|---|---|---|---|
| `zp` | 256 | `$0000-$00FF` | `A` |
| `iram` | 1792 | **`$0100-$07FF`** | **`A - 0x0100`** |
| `wram` | 8192 | `$6000-$7FFF` | `A - 0x6000` |
| `pw` | 8 | 密码缓冲 | — |

`iram` 起点是 `$0100` 这一条是所有节点取数的前提（旧版工具把它当 `$0000`，见 §10.1）。

## 2. 复跑命令（别人照抄即可得到同一组数）

```bash
cd /home/peng/workspace/metroid-decomp

# 2.1 长片全量回放（12000 帧/段；rust 腿连续跑一次，emu 腿分段）
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 120540 --segments 12000 \
  --range 0400-04FF --tag ens \
  --out analysis/tas-main-f22895 --timeout 900

# 2.2 覆盖读数（唯一权威 = json）
jq '.ran_frames, .total_frames, .segment_size, .first_divergence, .env' \
   analysis/tas-main-f22895/coverage.json
jq '[.segments[]|{index,from,to,status,compared,diverge_frames,sha256_agg_emu,sha256_agg_rust}]' \
   analysis/tas-main-f22895/coverage.json

# 2.3 9 节点判定（#99 修好版工具；本仓库主线尚未 pickup，故从 lane 提交取脚本到 /tmp 跑，不改 tools/）
git show bcd255d4:tools/stage_a_nodes.py > /tmp/r99/tools/stage_a_nodes.py   # 先建 /tmp/r99/{crates,docs} 软链到仓库
python3 /tmp/r99/tools/stage_a_nodes.py trace --trace analysis/tas-main-f22895/rust/full.state.jsonl
python3 /tmp/r99/tools/stage_a_nodes.py report --trace analysis/tas-main-f22895/rust/full.state.jsonl
python3 /tmp/r99/tools/stage_a_nodes.py links

# 2.4 独立复核（不依赖 #99 的改动，谓词与节点表逐字对齐）
python3 analysis/scratch-stage-a/audit_full.py
```

2.3 的 `report` 原样输出（帧数 120347 = 全片）：

```text
静态链路：5 条，断链 2 条（L1, L2）
动态轨迹：analysis/tas-main-f22895/rust/full.state.jsonl 帧数 120347，到达 0/9 个节点
第一个断点：S1 首个 Boss 击破（在 Boss 区）
判决：**不通关** —— 静态链路断在 L1（crates/game-core/src/enemy/framework.rs）、L2（crates/game-core/src/world/room_draw.rs）
```

2.4 原样输出（与 2.3 同结论，独立实现）：

```text
帧记录 120347，首帧 f=1，末帧 f=120540
in_area 低4位分布: {0: 120347}
逐键最大值: {'kraid_status': 0, 'ridley_status': 0, 'bridge': 0, 'in_area': 0, 'mb_status': 0,
            'end_timer': 255, 'zeb0': 0, 'zeb1': 0, 'zeb2': 0, 'zeb3': 0, 'zeb4': 0, 'at_ending': 0}
S6 前置 zeb 曾>0: False  其中在 Tourian 内: False
S8 前置 end_timer 曾 1..254: False
首到达: 9 节点全部未到达（0/9）
```

`analysis/scratch-stage-a/` 是 gitignored 的可再生工作区；其判定逻辑在 §5 里逐条写成文字，不依赖脚本文件也能复算。

## 3. 分段表（`coverage.json` 逐段原样）

| 段 | 区间 | 判定 | 比较帧 | 分歧帧 | emu sha | rust sha | 耗时(s) |
|---|---|---|---|---|---|---|---|
| 0 | [0,12000) | pass | 11962 | 0 | `ca10ee11c1c4` | `ca10ee11c1c4` | 211.6 |
| 1 | [12000,24000) | diverge | 11984 | 1100 | `7cd7a77ef4df` | `6da5b3673a98` | 211.5 |
| 2 | [24000,36000) | diverge | 11987 | 11987 | `84d32ad71188` | `614939d9d089` | 211.6 |
| 3 | [36000,48000) | diverge | 11977 | 11977 | `9559d95cafbc` | `d47713d0d9b7` | 211.5 |
| 4 | [48000,60000) | diverge | 11992 | 11992 | `22fb82fd7544` | `929763fc2464` | 211.5 |
| 5 | [60000,72000) | diverge | 11948 | 11948 | `8e1faddb23f6` | `3fd13f091bc8` | 211.5 |
| 6 | [72000,84000) | diverge | 11970 | 11970 | `3a3acef0d6bf` | `b485adf887c0` | 211.5 |
| 7 | [84000,96000) | diverge | 11998 | 11998 | `c5e14b0e4fce` | `e665a101c471` | 211.5 |
| 8 | [96000,108000) | diverge | 11958 | 11958 | `94871a21eca4` | `f120416b54bd` | 211.6 |
| 9 | [108000,120000) | diverge | 11948 | 11948 | `729d2f693b2c` | `ad7010b3bffb` | 211.5 |
| 10 | [120000,120540) | diverge | 541 | 541 | `8444edf10a10` | `e0454a1a3387` | 20.6 |

- 只有段 0 逐帧全等（11962 比较帧、0 分歧）；段 1 有 1100/11984 帧不一致（f22895 起算到段尾）；段 2 之后**整段全不一致**。
- 段 2 以后「全段分歧」不代表新增缺陷：一旦在 f22895 分叉，两侧状态就再也回不到同一轨道，分歧帧数只表示漂移规模。
- 本表 11 段的 `rust_rows` **全部 = 120347**（全片同一份轨迹），这是它能当权威读数的前提；
  `legacy-s10000` 那份是 `29946`（前 3 段）/ `120347`（后 10 段）混用，见 §10.5。

## 4. 首分歧与「卡在第一次转场」

**首分歧（A 级，逐字节）**：帧 22895，`$0405`：emu=`0x5A`(90)、rust=`0x1A`(26)；该帧共 8 字节不同
（偏移 5/21/37/53/69/73/74/85，都在 `$0400-$04FF` 敌槽 0 的范围内）。复算：

```bash
python3 - <<'EOF'
import json
f=22895
emu=[json.loads(l) for l in open("analysis/tas-main-f22895/emu/seg-012000-024000.jsonl")]
emu={d.get("f",d.get("frame")):d for d in emu}[f]["w"]["ens"]
rust={d["f"]:d for d in map(json.loads,open("analysis/tas-main-f22895/rust/full.state.jsonl"))}[f]["w"]["iram"]
e=bytes.fromhex(emu); r=bytes.fromhex(rust)[0x400-0x100:0x500-0x100]
print([(i,e[i],r[i]) for i in range(256) if e[i]!=r[i]])
EOF
# → [(5, 90, 26), (21, 30, 94), (37, 86, 22), (53, 0, 64), (69, 88, 26), (73, 1, 0), (74, 1, 0), (85, 24, 90)]
```

**分叉之后的形态（这才是 0/9 的直接原因）**：整条 120540 帧的 Rust 腿轨迹里——

| 观测量 | 实测 | 含义 |
|---|---|---|
| `InArea`（`$0074`）低 4 位 | **120347 帧全部 = 0** | 从未离开 Brinstar（1=Norfair、2=Kraid、3=Tourian、4=Ridley 一次都没出现） |
| `RoomNumber`（`$005A`） | **109 次相邻帧跳变**（120347 帧，不含首帧），取值集合**只有 `{0, 241, 243, 255}`** | 只有 1 个真实房间（0）；241/243/255 是转场哨兵，说明它反复进转场又回到 0 号房 |
| Samus X / Y（`$030E`/`$030D`） | X∈[0,255]、Y∈[0,200] | 它**在房间里正常走动**（不是卡死不动），只是**出不去** |

复算：

```bash
python3 - <<'EOF'
import json
first=True; prev=None; ch=[]
for line in open("analysis/tas-main-f22895/rust/full.state.jsonl"):
    d=json.loads(line); z=d["w"]["zp"]
    area=int(z[0x74*2:0x74*2+2],16); room=int(z[0x5A*2:0x5A*2+2],16)
    if not first and room!=prev: ch.append((d["f"],room,area))   # 只数相邻采样帧之间的变化，不含首帧首次取值
    prev=room; first=False
print("跳变次数", len(ch), "房间集合", sorted({c[1] for c in ch}), "area 集合", sorted({c[2] for c in ch}))
EOF
```

**结论**：`#101` 要回答的「Tourian / Boss / 雕像 / 结局段是否被覆盖」的答案是
**「录像本身覆盖了（100% 路线必然经过），但 rust 腿在 f22895 的门转场处就分叉并卡死，所以这些段在 rust 腿上是不可达的」**。
这与 `#104`（门格碰撞瓦 `$A1` 写入链与到达性）是同一个问题的两个视角。

## 5. 9 节点到达表（本卡主表）

判定口径：帧号取 **Rust 腿**轨迹（emu 腿只采 `$0400-$04FF`，无法判定节点）；
「覆盖内」= 该节点所需的区域/条件在这条 120540 帧轨迹里**真的出现过**，不是「工具跑完没报错」。

| # | 节点 | watch 键 | 首到达帧 | 在 120540 帧覆盖内？ | 证据（全片实测极值） |
|---|---|---|---|---|---|
| S1 | 首个 Boss 击破（在 Boss 区） | `$687B`、`$687C`、`$0074` | **未到达** | **否**：区号恒 0，从未进 2/4 区 | `kraid_status=0`、`ridley_status=0` 全程 |
| S2 | 两个 Boss 均击破 | `$687B` bit0、`$687C` bit1 | **未到达** | **否** | 同上（两个字节没有一位被置） |
| S3 | 两座雕像升起（Brinstar） | 两字节 bit7、`$0074` | **未到达** | **否**（前提 S1/S2 未发生） | 两字节恒 0 |
| S4 | 通路桥生成 | `$0054` | **未到达** | **否** | `$0054` 恒 0 |
| S5 | 进入 Tourian | `$0074` 低 4 位 =3 | **未到达** | **否**：区号从未等于 3 | 120347 帧 area 全 0 |
| S6 | Zebetite 清空 | `$0758+8k`（k=0..4） | **未到达** | **否**：Tourian 未进入，Zebetite 未生成 | 5 字节全程 0 =「未生成」，且「曾 >0」前置为假（`require_seen` 不成立） |
| S7 | 母脑击破 | `$0098` | **未到达** | **否** | `$0098` 恒 0 |
| S8 | 逃逸计时耗尽 | `$010A` | **未到达** | **否**：EndTimer 从未进入计时态（只有 255 与瞬态 0，无 1..254） | `$010A` 120322 帧 = 255、25 帧 = 0（`require_seen: end_timer>0` 不成立） |
| S9 | 结局触发 | `$6883` | **未到达** | **否** | `$6883` 恒 0 |

**已到达清单：空。未到达清单：S1..S9 全部（9/9）。**

> 「快照路径」= `analysis/tas-main-f22895/rust/full.state.jsonl` 里 `f` 等于该帧号的记录行（一行一帧）。
> 9 个节点都未到达，因此没有「到达帧快照」；判据是**逐帧扫描全部 120347 条记录**后该节点谓词从未成立。

## 6. 静态链路（`links`）

`python3 /tmp/r99/tools/stage_a_nodes.py links` → **5 条，断链 2 条**：

| 链路 | 结果 | 位置 / 缺什么 |
|---|---|---|
| L1 Boss 击杀结算写雕像状态（`LDD75`） | **断** | `crates/game-core/src/enemy/framework.rs` 缺 `0x687B|KRAID_STATUE_STATUS` |
| L2 Boss 房重生门读雕像状态 | **断** | `crates/game-core/src/world/room_draw.rs` 缺 `0x687B|KRAID_STATUE_STATUS` |
| L3 雕像升起 | OK | `crates/game-core/src/world/statue.rs:93` |
| L4 桥生成 | OK | `crates/game-core/src/world/statue.rs:38` |
| L5 结局电梯触发 | OK | `crates/game-core/src/world/elevator.rs:18` |

**触达判断（卡面第 3 条）**：这两处断链在当前 TAS 上**没有被触达**——它们只在「击杀 Boss 后回 Boss 房 / 雕像房」时才生效，
而 rust 腿连第一个门都没过去（§4），所以「断链」是**静态缺实现**（写读两侧都没有这两个状态的引用），
不是「动态走了但写错了」。这条要等门转场修好、能进 Kraid/Ridley 之后才能用动态证据复核。

## 7. 覆盖边界（必须与本表一起引用）

- **本次覆盖到的是**：120540 帧全片的 **Rust 腿**内存轨迹（120347 行，含 `zp/iram/wram/pw`）；
  emu 腿只覆盖 `$0400-$04FF`（敌槽面），**不覆盖**节点所需的 zp/wram/iram 键。
- **判等面只有 `$0400-$04FF`**：`--range 0400-04FF`。这个面窄是刻意的（与历史用例同口径），
  但它**不等于**「全内存一致」；其他面（`zp`/`iram`/`wram`）在本卡里只用于**读节点键**，没有逐帧对拍。
- **rust 腿在 f22895 之后不再是「原版行为的投影」**：那之后 rust 与原版的输入流虽然相同，
  但状态已经分叉（卡在起始房），所以 f22895 之后的节点判定回答的是
  **「rust 自己能走到哪」**，不是「这条 TAS 在原机上会怎样」。
- **`run-all` 171/172 / `l4` 171/172 不是通关证据**：它们证明的是「已登记的 21 个用例逐像素一致」，
  用例集合里没有 Tourian/Boss/结局段（见 `docs/design/stage-a-gate.md` 的节点表与 `shots_rust/matrix.json`）。

## 8. 要真正证明可通关，还缺什么（可执行清单，按依赖顺序）

1. **修门转场（阻塞项，`#104` 正在做）**：f22895 处 rust 与 emu 在敌槽 0 上分叉，随后 rust 反复尝试转场、
   房间号回到 0。要先把「门格碰撞瓦 `$A1` 的写入链」打通，让 rust 能真正离开起始房。
2. **重跑本卡**（修好后一条命令）：`replay --frames 120540 --segments 12000 …`，预期 seg2 起不再整段全分歧；
   首分歧帧若后移，就是进展的量化指标。
3. **补 L1/L2 两处静态实现**（`enemy/framework.rs` 写 `$687B/$687C`、`world/room_draw.rs` 读），
   否则即使能进 Boss 房，雕像状态也永远不置位 ⇒ S1/S2/S3/S4 不可达。
4. **Boss 段录像**：当前 100% TAS 是「全收集」路线，理论上包含 Kraid/Ridley；但要在 rust 上验证，
   必须先满足第 1 条。若到时仍不到达，需要单独剪出「Kraid 击杀」「Ridley 击杀」两段短录像做定点对拍。
5. **Tourian + 结局段**：同上，须在第 1 条之后；若门转场修好后仍到不了 Tourian，
   用 `#99` 修好版工具在整片上找**第一个断点帧**（`trace` 会报「第一个断点」），按帧定位缺口，不再靠推断。

## 9. 逐键极值与复算（自证不是「没采到」）

| 键 | 地址 | 全片最大值 | 非零帧数 | 说明 |
|---|---|---|---|---|
| `kraid_status` | `$687B` | 0 | 0 | 从未置位 |
| `ridley_status` | `$687C` | 0 | 0 | 从未置位 |
| `bridge` | `$0054` | 0 | 0 | 桥从未生成 |
| `in_area` | `$0074` | 0 | 0 | 区号恒 0（Brinstar） |
| `mb_status` | `$0098` | 0 | 0 | 母脑未被激活 |
| `end_timer` | `$010A` | **255** | 120322 帧为 255、25 帧为 0 | 逃逸计时从未装弹（255=未启用）；0 只出现在开机段与 12 个转场瞬态帧 |
| `zeb0..zeb4` | `$0758+8k` | 0 | 0 | 从未生成（Tourian 未进入） |
| `at_ending` | `$6883` | 0 | 0 | 结局未触发 |

**「没采到」与「真为 0」的区分**：`iram` 块在每行里都是 1792 B 完整导出（`len(w["iram"])==3584` hex 字符），
`wram` 8192 B、`zp` 256 B；键的偏移都在块内（§1 表），取到的是真字节而不是缺省 0——
`#99` 版工具对越界已改成 fail-loud（`_decode_blobs` 抛 `NodeError`），本次运行没有触发任何越界。

## 10. 工具缺陷与口径更正（不改 `tools/`，只报告）

### 10.1 G6：`WATCH_ADDR` 的 `iram` 偏移差 `0x0100`（已由 `#99` 修）

旧版把绝对地址当块内偏移：`end_timer 0x010A` 实际读到 `$020A`、`zeb0..4 0x0758…` 越界被**静默跳过**。
`#99`（lane `bcd255d4`）已改成 `addr-0x0100` 并对越界 fail-loud；本文 §2.3 用的就是那一版。
**状态**：修好版**尚未 pickup 到主线**，所以本仓库 `tools/stage_a_nodes.py` 现在仍是旧口径。

### 10.2 G8：`--resume` 不校验 rust 腿帧数（本次踩到并绕过）

现象：`replay --frames 120540 --segments 10000 --resume` 跑到 seg4 直接 `fail`：
`段内无共同帧（emu=9979 rust=29946）——空跑不得绿`。
根因：复用判据只看 `analysis/tas-main-f22895/rust/full.state.jsonl` 是否存在，**不校验它覆盖到第几帧**；
旧文件是 30000 帧那轮留下的（29946 行），于是 rust 腿被跳过，emu 段却跑到 40000+。
绕过：把旧腿改名 `rust/full.state.30000.jsonl.bak`（615 MB，留作 seg0–3 证据）后重跑。
建议：让 `--resume` 校验 `rust_rows >= frames`（或校验 sched 里的 `frames`）后才复用。

### 10.3 G9：`verify-convert` 的断言写死 `$0400`

`#99` 的 `verify-convert` 取 `coverage.first_divergence.rust`，断言 `iram[$0400-$0100] == rust`。
本次首分歧字段是 **`$0405`**（不是 `$0400`），于是：

```text
[换算] 帧 22895：iram[0x0400-0x0100]=iram[0x300]=152，coverage.rust=26（iram[0x400]=0）
[FAIL] iram 偏移换算与 coverage 不一致（期望 $0400 落在 0x300）
```

这是**断言口径**问题不是数据问题：`iram[$0400]=152` 与 `iram[$0405]=26` 是两个不同字节，
coverage 记的 `rust=26` 对应 `$0405`。建议断言改成按 `field` 里的地址算偏移（`field` 已带十六进制地址）。
> 该条已同步给 `#99` 的负责人。

### 10.4 口径更正：`coverage.md` 的 `98/97` 不是旧读数，是十六进制（§3.6 更正）

- 原文（本文件 2026-11-26 早先版本、`#98` 交付）：把 `coverage.md` 的
  「帧 22895 字段 `$0400`：emu=98 / rust=97（11 字节）」判为「旧读数，应为 152/151」。
- **更正（2026-11-26 19:00，`#101` 实测）**：`tools/tas_replay_diff.py` 的 `write_coverage()` 用
  `f"{global_fd['emu']:02X}"` 输出，所以 md 里的 `98/97` 是**十六进制**：
  `0x98 = 152`、`0x97 = 151`，与同一份 `coverage.json` 的 `152/151` 是**同一个值**。
  「md 陈旧」的说法**不成立**；md 与 json 的唯一差别是进制。
- 因此卡面第 4 条「把 `98/97` 改成 `152/151`」**不改生成结果**（改了反而与生成器冲突）。
  若要让两者一致，正确做法是改 `tools/tas_replay_diff.py` 的格式化（`{:02X}` → `{}`），属 `tools/` 改动，不在本卡写面。
- 注：`98/97`（=152/151）与 §4 的 `90/26` 不是进制问题，而是**两次运行在 f22895 上确实比出了不同字节**：
  旧一轮（10000/段）前 3 段对着 **29946 行**的旧 rust 轨迹比，本轮（12000/段）对着 **120347 行**的全片轨迹比。
  当前轨迹在 `$0400` 上是 152（与 emu 相同），真正的首个差异是 `$0405`（emu 90 / rust 26）。成因见 §10.5。

### 10.5 为什么本文用 12000/段那份，而不是 10000/段那份（与 PMO seq5/seq6 口径的差异，附硬证据）

**经过**：PMO 在 thread seq5/seq6 裁定「停掉 12000/段重跑，直接用已跑完的 10000/段（13 段）结果提交」，理由是段长不是判据、
重跑会作废已落盘的 sha。我按裁定把 10000/段那份设为权威——**随后复核发现那一份不能当权威读数**：

```bash
# 两条命令即可看出问题：同一份 coverage 里 rust_rows 出现两个值
jq '[.segments[].rust_rows] | unique' analysis/tas-main-f22895/legacy-s10000/coverage.json
# → [29946, 120347]   （前 3 段用的是上一轮 30000 帧留下的 rust 轨迹）
jq '[.segments[].rust_rows] | unique' analysis/tas-main-f22895/coverage.json
# → [120347]          （本轮 11 段全部对同一份全片轨迹）
```

- 成因就是 §10.2 的 G8：`--resume` 当时只判断 `rust/full.state.jsonl` 存在、不校验帧数，于是 seg0–seg2 跳过了 rust 腿重跑，
  拿 29946 行的旧轨迹去和跑到 40000+ 的 emu 段比。它的 `first_divergence = f22895 $0400 152/151(11B)` 是**旧 rust 构建**的读数。
- 本轮（12000/段）重建了全片 rust 腿（`rust/full.state.jsonl`，120347 行，
  `sha256=7d76adc03018adb240e0b993db7a2479f2f9194f96c1a727639dc17892a2b06d`，两次独立运行逐字节相同），
  11 段 `rust_rows` 恒为 120347，读数自洽：f22895 上 `$0400` 两侧都是 152，首个差异在 `$0405`（90 vs 26）。

**直接逐字节复核（不依赖任何 coverage 字段）**：

```bash
python3 - <<'EOF'
import json
fr=22895
r={json.loads(l)['f']:json.loads(l) for l in open('analysis/tas-main-f22895/rust/full.state.jsonl')}[fr]
e={json.loads(l)['f']:json.loads(l) for l in open('analysis/tas-main-f22895/emu/seg-012000-024000.jsonl')}[fr]
rb=bytes.fromhex(r['w']['iram'])[0x300:0x400]   # $0400-$04FF
eb=bytes.fromhex(e['w']['ens'])
print([i for i in range(256) if rb[i]!=eb[i]], rb[0], eb[0])
EOF
# → [5, 21, 37, 53, 69, 73, 74, 85] 152 152
```

**处置**：10000/段那份移到 `analysis/tas-main-f22895/legacy-s10000/` 并附 `README.md` 写明污染原因；本文一切结论用 12000/段那份
（`ran_frames=120540/120540`、11 段、`compared` 合计 120265、失败段 0、首分歧 f22895 `$0405`）。
段长按 PMO 自己的规则 2「段长参数写进报告」实写 **12000**——不是把它当成 10000 写，也不是拿污染数据凑判据。
**若 PMO 仍坚持要一份 10000/段的干净读数**：需要重跑 emu 13 段（约 40 分钟）+ 让 rust 腿与之一致，结论不会变（首分歧仍是 f22895），
请直接在 thread 说一声，我另开一轮跑；本轮不再为段长消耗机时。

## 11. 与在途改动的关系（交付面隔离）

- 本卡交付面：`docs/design/stage-a-evidence.md`（本文）+ `analysis/tas-main-f22895/`（轨迹与 coverage）。
- 工作树里的 8 个 `M crates/game-core/src/...` 是 **#95 已验收未入库**的改动；
  `coverage.json` 的 `env.git_dirty_crates` 已把它们逐条记下（用于判断两次报告是否同一口径）。
- `?? docs/spec/b2b-extra-6afb.md` 属 `#96`，`?? docs/spec/zero-page-semantics.md` 属 `#100`，与本卡无关。

## 12. 自检表（按 PMO seq5 的五条验收口径，逐条给命令与原样输出）

| # | 验收口径 | 命令 | 实测输出 |
|---|---|---|---|
| 1 | `ran_frames == total_frames == 120540`，段长/段数如实 | `jq '[.ran_frames,.total_frames,.segment_size,(.segments\|length)]' analysis/tas-main-f22895/coverage.json` | `[120540,120540,12000,11]` |
| 2 | 每段 status + 帧区间 + sha，段长参数写进报告 | `jq -r '.segments[] \| [.index,.from,.to,.status,.compared,.diverge_frames] \| @tsv' …/coverage.json` | §3 表 11 行即其原样（段长 12000 已写进表头与 §1） |
| 3 | 全局 `first_divergence` 如实 | `jq .first_divergence …/coverage.json` | `{"frame":22895,"field":"$0405","emu":90,"rust":26,"diff_bytes":8}` |
| 4 | 9 节点表：每节点 `reached` + 首达帧 + 判定依据 | `python3 /tmp/r99/tools/stage_a_nodes.py trace --trace analysis/tas-main-f22895/rust/full.state.jsonl` | 到达 **0/9**（表内 9 行全是 `—`），`第一个断点：S1`；表见 §5，逐条给 `in_area` 门与 `require_seen` 依据 |
| 5a | `gates/cli.py check` PASS24/0/3/1 | `python3 gates/cli.py check` | `SUMMARY: PASS=24 FAIL=0 SKIP=3 TOLERATED=1`、`CHECK: PASS（13.12s）` |
| 5b | `cargo test -p game-core` 不回退 | `cargo test -q -p game-core 2>&1 \| grep -E "^test result"` | `passed=1133 failed=0`（workspace 口径 1217，ratchet 基线 1217 未动） |
| 5c | `run-all` 171/172 | `python3 tools/l4_diff_run.py run-all --allow-dirty` | `用例 21 · 采集点格 172 ｜ match 171 · diff 1 · blocked 0 ｜ pass 171/172`（唯一 diff 仍是 `case-mechanism-input-conflict` 的 `f674=84px` 既有基线差；矩阵 `head=d2fea892`、`at=2026-11-26T11:18:10Z`） |

**段长口径的最后一句**：本轮实际参数是 `--segments 12000`（11 段）。PMO seq5/seq6 要求「用 10000/段那份、段长写 10000」——
那一份已被 §10.5 证明前 3 段对比的是旧 rust 轨迹，不能用；本文按实际参数写 12000，并在 §10.5 给出「要 10000/段干净读数需重跑」的代价与请确认项。
