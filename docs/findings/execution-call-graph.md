# findings：boot/title/samus-move 执行路径调用图——执行序端口队列（task-12）

> 状态：**已证实**（2026-10-15，Mesen2 动态 × 静态 xref 双工具交叉）。换时刻双跑全同。
> 复跑链（可复现）：
> ```sh
> python3 tools/decode_sweep.py --json /tmp/sweep.json
> python3 tools/call_graph.py anchors --out analysis/_gen/anchors.json --static /tmp/sweep.json
> python3 tools/scenario_run.py run execpath-boot    # entry_probe 探针在场景内
> python3 tools/scenario_run.py run execpath-move
> python3 tools/call_graph.py merge --run analysis/traces/execpath-boot \
>     --run analysis/traces/execpath-move --phase boot,move --static /tmp/sweep.json
> ```

## 0. 方法（双工具交叉 + 三查）

- **动态（A 级）**：`entry_probe`（scenario DSL 新指令）= 711 个函数级锚点
  （seed sub_ 331 + decode_sweep 有佐证岛入口 378 + 根 2）各挂 `callbackType.exec`
  真指令入口回调（r3 更正语义）——逐入口聚合执行序（全局 seq）/首帧/命中数/调用者边；
- **静态（A 级）**：seed 代码项 ROM 字节 xref（JSR/JMP 1056 边）+ decode_sweep 岛内流
  调用点 + C27C 跳转表词边；
- **交叉**：动态 jsr 边须过 ROM 字节核（site 处 `20 lo hi` 且 word==callee）——
  188 条栈帧残留伪边剔除后 345 真边；真边静态未见 55（登记）；静态边未动态出现 2
  （路径受限，非否定结论）；
- **换时刻**：execpath-boot 双跑入口集 43/43 全同（仅1=0 仅2=0）；
- **调用深度（B 级推导）**：函数级图 BFS（根=C0D9 NMI/FFB0 RESET；IDA funcs 跨度做
  site→函数入口包含映射 + 函数顺流边 A.end==B.start + xref/跳转表边）——d=-1 仅 16
  （间接可达/未定，如实标注）。

## 1. 总量（输出原文）

```
[call_graph] 运行 2 个场景（['boot', 'move']）；入口 264，动态边 562（jsr 345），静态边 1056
[call_graph] 双工具交叉：动态 jsr 边静态未见 55；静态边未在动态出现（路径受限）2
[call_graph] 换时刻双跑：入口集 跑1 43 跑2 43 仅1 0 仅2 0
```

- boot/title 相命中 **43** 入口（300 帧）；samus-move 相新增至 **264**（1400 帧）；
- 队列=按 (相序, first_seq) 执行序排——**附录 A 全量 264 行**（seq|bank:addr|深度|域|首执行|命中|入边）。

## 2. 执行序骨架（boot 相前 35，节选——喂 task-10 段一节奏）

| seq | 节点 | 深度 | 首执行 | 命中 | 要点 |
|---|---|---|---|---|---|
| 1-11 | $C158/$C16D/$C175/$C4B2/$C4DE/$C4E8/$C4EF/$C4FA/$C45D/$C43D/$C434 | 2-11 | f0 | — | **reset 初始化链**（f0=复位后、首 NMI 前） |
| 12 | $C0D9 | 0 | f1 | 1698 | **NMI 根** |
| 13 | $C1E0 | 1 | f1 | 1688 | NMI 一级 |
| 16/18/21/22/24/26 | $C2A8/$C2CA/$C29A/$C44D/$C215/$C97E | 1 | f1 | ~1688 | NMI 每帧一级面 |
| 14/17 | $C36B/$C376 | 4 | f1 | 1464/117 | 二级族 |
| 31/32/33/34 | $C000/$C0BC/$C266/$C114 | 10/-1/3/3 | f1 | 1678 | **每帧调用族**（C0BC=复位顺流尾；C266=timer 族） |
| 35 | $8000（窗口） | 2 | f1 | 948 | bank0 标题窗口入口 |
| 36 | $C27C | — | f1 | 8277 | **分发器**（r3 A 值域表） |

move 相新增自 seq 44：$909F/$90D1（解锁链 f360-362）→ f542 进游戏（$C1D4/$C578/$C064/$CA18
…）→ f545 移动主链（$F1F4 n=17052、$E193 n=17046、$C2BF/$C2C0 查表族、$EA2B、$EB85…）。
**喂 task-11 的移动面核心**：$F1F4/$E193/$C2BF/$C2C0/$EA2B/$EB85 高频面。

## 3. 双工具交叉登记（否定性结论三查）

- 动态真边静态未见 55（附录 B）：主因=窗口地址跨 bank 语义（site 在 0x8000 窗口，
  bank 归属多解）与分发蹦床边——逐条登记不隐没；
- 静态边未在动态出现 2：路径受限（场景未覆盖），非「不存在」；
- 0 命中类结论一律「路径受限」标注（本链场景=boot/title/单房走打；门/Boss/道具
  路径未覆盖）。

## 4. 工程修正（随本任务）

- **P7-F2 fail-loud**：decode_sweep_dyn 对不存在 traces 前缀静默按 0（「空跑绿」族）→
  改 FATAL 断言（实测：`FATAL: traces 前缀无任何产物文件: analysis/traces/nonexistent.*.jsonl`）；
- entry_probe DSL（tools/scenario_run.py + scenario_capture.lua）：711 锚点单地址
  注册（callbackType.exec），执行序/边聚合落 `<name>.entries.jsonl`/`edges.jsonl`；
- 深度图三修实录：site 边需函数包含映射（IDA funcs 跨度）+ 函数顺流边（FFB0→C01A→
  C0BC 断链根因）——d=-1 从 246 收敛至 16。

## 5. 证据分级

| 结论 | 级 | 依据 |
|---|---|---|
| 264 入口执行序队列 | **A** | callbackType.exec 真执行 + 换时刻双跑全同 |
| 345 动态真 jsr 边 | **A** | ROM 字节核（site `20 lo hi`==callee） |
| 调用深度（0-11 分布） | **B** | A 级边 + IDA funcs 包含映射推导；含映射启发式 |
| 55 静态未见边 | **B** | 窗口 bank 归属多解，登记待解 |
| 静态 2 边未执行 | — | 路径受限（显式，非否定结论） |

## 6. 遗留

1. 55 条静态未见边的窗口归属消歧（需 bank 切换上下文跟踪——mapper.prgReg 时间线可补）；
2. d=-1 16 节点（间接可达：跳转表/蹦床深链）；
3. 门/Boss/道具场景路径扩链（队列会随场景增长——本队列=boot/title/move 三相闭环）；
4. porter C01A 解码草图落库后可作队列域标注的语义输入（lead 转告，待指针）。

## 附录 A（机械生成）：执行序端口队列全量 264 行

生成命令：`python3 tools/call_graph.py merge …`（见文首复跑链；输出与本附录同源）。
列：seq | bank:addr | d=调用深度（-1=间接/未定） | 域（sub/root/entry@相） | 首执行帧 |
命中 n | 入边数。

## 附录 B（机械生成）：动态真边静态未见（merge 输出节选前 20 / 共 55 条）

全量 55 条以重跑命令输出为准（merge 按 20 条节选打印）。

### A.1 全量队列（机械生成原文）
```
1 | bank7:$C158 | d=2 | sub@boot | f0 | n=8 | in=5
2 | bank7:$C16D | d=3 | sub@boot | f0 | n=8 | in=1
3 | bank7:$C175 | d=4 | sub@boot | f0 | n=16 | in=6
4 | bank7:$C4B2 | d=11 | sub@boot | f0 | n=2 | in=1
5 | bank7:$C4DE | d=3 | sub@boot | f0 | n=1680 | in=2
6 | bank7:$C4E8 | d=4 | sub@boot | f0 | n=3 | in=1
7 | bank7:$C4EF | d=5 | sub@boot | f0 | n=44 | in=16
8 | bank7:$C4FA | d=6 | sub@boot | f0 | n=44 | in=16
9 | bank7:$C45D | d=2 | sub@boot | f0 | n=3 | in=2
10 | bank7:$C43D | d=3 | sub@boot | f0 | n=11 | in=5
11 | bank7:$C434 | d=4 | sub@boot | f0 | n=12 | in=2
12 | bank7:$C0D9 | d=0 | root@boot | f1 | n=1698 | in=28
13 | bank7:$C1E0 | d=1 | sub@boot | f1 | n=1688 | in=1
14 | bank7:$C36B | d=4 | sub@boot | f1 | n=1464 | in=10
15 | bank7:$C3C6 | d=5 | sub@boot | f1 | n=113 | in=2
16 | bank7:$C2A8 | d=2 | sub@boot | f1 | n=1506 | in=4
17 | bank7:$C376 | d=4 | sub@boot | f1 | n=117 | in=4
18 | bank7:$C2CA | d=1 | sub@boot | f1 | n=1688 | in=1
19 | bank7:$C30C | d=2 | sub@boot | f1 | n=316 | in=4
20 | bank7:$C318 | d=2 | sub@boot | f1 | n=214 | in=1
21 | bank7:$C29A | d=1 | sub@boot | f1 | n=1790 | in=5
22 | bank7:$C44D | d=1 | sub@boot | f1 | n=1688 | in=1
23 | bank7:$C4D9 | d=2 | sub@boot | f1 | n=1688 | in=1
24 | bank7:$C215 | d=1 | sub@boot | f1 | n=1688 | in=1
25 | bank7:$C21F | d=2 | sub@boot | f1 | n=3376 | in=2
26 | bank7:$C97E | d=1 | sub@boot | f1 | n=1698 | in=1
27 | bank7:$C5D7 | d=7 | sub@boot | f1 | n=2 | in=1
28 | bank7:$C7AB | d=8 | sub@boot | f1 | n=20 | in=13
29 | bank7:$C7D5 | d=9 | sub@boot | f1 | n=20 | in=1
30 | bank7:$C487 | d=7 | sub@boot | f1 | n=4 | in=3
31 | bank7:$C000 | d=10 | sub@boot | f1 | n=1678 | in=1
32 | bank7:$C0BC | d=-1 | sub@boot | f1 | n=1678 | in=1
33 | bank7:$C266 | d=3 | sub@boot | f1 | n=1678 | in=1
34 | bank7:$C114 | d=1 | sub@boot | f1 | n=1678 | in=1
35 | bank?:$8000 | d=2 | sub@boot | f1 | n=948 | in=2
36 | bank7:$C27C | d=2 | sub@boot | f1 | n=8277 | in=11
37 | bank?:$80C5 | d=5 | sub@boot | f2 | n=2 | in=1
38 | bank?:$98AE | d=6 | sub@boot | f2 | n=2 | in=1
39 | bank7:$C439 | d=2 | sub@boot | f3 | n=4 | in=3
40 | bank7:$C447 | d=4 | sub@boot | f5 | n=4 | in=1
41 | bank?:$80FB | d=3 | sub@boot | f7 | n=144 | in=2
42 | bank?:$8134 | d=3 | sub@boot | f85 | n=264 | in=2
43 | bank?:$8A8C | d=4 | sub@boot | f85 | n=18 | in=1
44 | bank?:$909F | d=13 | sub@move | f360 | n=1 | in=1
45 | bank?:$90D1 | d=-1 | sub@move | f362 | n=1 | in=1
46 | bank7:$C1D4 | d=7 | sub@move | f542 | n=1 | in=1
47 | bank7:$C578 | d=7 | sub@move | f542 | n=1 | in=1
48 | bank7:$C604 | d=7 | sub@move | f542 | n=1 | in=1
49 | bank7:$CA18 | d=10 | sub@move | f542 | n=1 | in=1
50 | bank7:$C8BB | d=10 | sub@move | f544 | n=1 | in=1
51 | bank7:$C8B0 | d=10 | sub@move | f545 | n=1 | in=1
52 | bank7:$E720 | d=8 | sub@move | f545 | n=2 | in=2
53 | bank7:$EC93 | d=9 | sub@move | f545 | n=2 | in=1
54 | bank7:$C2C5 | d=4 | sub@move | f545 | n=829 | in=4
55 | bank7:$C2C6 | d=4 | sub@move | f545 | n=829 | in=4
56 | bank7:$EA2B | d=9 | sub@move | f545 | n=366 | in=2
57 | bank7:$EC9B | d=10 | sub@move | f545 | n=2 | in=1
58 | bank7:$ED51 | d=11 | sub@move | f545 | n=4 | in=2
59 | bank7:$EB85 | d=11 | sub@move | f545 | n=44 | in=6
60 | bank7:$F1F4 | d=7 | sub@move | f545 | n=17052 | in=10
61 | bank7:$ED65 | d=11 | sub@move | f545 | n=2 | in=1
62 | bank7:$ED5B | d=11 | sub@move | f545 | n=2 | in=1
63 | bank7:$ED7A | d=11 | sub@move | f545 | n=6 | in=3
64 | bank7:$ED8C | d=11 | sub@move | f545 | n=4 | in=2
65 | bank7:$EF09 | d=11 | sub@move | f545 | n=11 | in=3
66 | bank7:$EF00 | d=12 | sub@move | f545 | n=2 | in=1
67 | bank7:$EAC0 | d=10 | sub@move | f545 | n=15 | in=2
68 | bank7:$EA05 | d=10 | sub@move | f545 | n=2 | in=1
69 | bank7:$EFF8 | d=10 | sub@move | f545 | n=2 | in=1
70 | bank7:$F01C | d=11 | sub@move | f545 | n=2 | in=1
71 | bank7:$E193 | d=6 | sub@move | f545 | n=17046 | in=4
72 | bank7:$C2BF | d=4 | sub@move | f545 | n=4328 | in=8
73 | bank7:$C2C0 | d=5 | sub@move | f545 | n=4357 | in=9
74 | bank7:$EF8C | d=10 | sub@move | f545 | n=49 | in=1
75 | bank7:$EF9E | d=12 | sub@move | f545 | n=170 | in=1
76 | bank7:$EB06 | d=11 | entry@move | f547 | n=5 | in=2
77 | bank7:$EB0C | d=12 | entry@move | f547 | n=5 | in=1
78 | bank7:$EB7A | d=13 | entry@move | f547 | n=5 | in=1
79 | bank7:$EB28 | d=13 | sub@move | f547 | n=3 | in=1
80 | bank7:$EB4D | d=13 | entry@move | f547 | n=3 | in=1
81 | bank7:$FB7B | d=14 | sub@move | f547 | n=3 | in=1
82 | bank?:$80B0 | d=4 | sub@move | f547 | n=259 | in=8
83 | bank7:$E564 | d=-1 | sub@move | f547 | n=33 | in=2
84 | bank7:$CBFD | d=6 | sub@move | f549 | n=1 | in=1
85 | bank7:$CF6B | d=2 | sub@move | f686 | n=197 | in=6
86 | bank7:$CF6E | d=3 | sub@move | f686 | n=197 | in=6
87 | bank7:$DE47 | d=2 | sub@move | f686 | n=150 | in=1
88 | bank7:$DC8F | d=3 | sub@move | f686 | n=562 | in=2
89 | bank7:$DE4A | d=3 | sub@move | f686 | n=602 | in=4
90 | bank7:$DCC3 | d=4 | sub@move | f686 | n=1266 | in=2
91 | bank7:$DE3D | d=4 | sub@move | f686 | n=1266 | in=2
92 | bank7:$DF6B | d=4 | sub@move | f686 | n=7228 | in=1
93 | bank7:$DFA3 | d=4 | sub@move | f686 | n=7228 | in=1
94 | bank7:$DF2D | d=4 | sub@move | f686 | n=1291 | in=5
95 | bank7:$D92C | d=2 | sub@move | f985 | n=1 | in=1
96 | bank7:$CB73 | d=2 | sub@move | f985 | n=1 | in=1
97 | bank7:$E1F1 | d=8 | sub@move | f986 | n=824 | in=2
98 | bank7:$CB29 | d=11 | sub@move | f986 | n=412 | in=1
99 | bank7:$F345 | d=12 | sub@move | f986 | n=412 | in=1
100 | bank7:$F351 | d=13 | sub@move | f986 | n=2472 | in=2
101 | bank7:$F37F | d=14 | sub@move | f986 | n=1082 | in=1
102 | bank7:$F3AA | d=14 | sub@move | f986 | n=2080 | in=1
103 | bank7:$F3BE | d=14 | entry@move | f986 | n=44 | in=1
104 | bank7:$F6B9 | d=15 | entry@move | f986 | n=346 | in=2
105 | bank7:$F74B | d=5 | sub@move | f986 | n=456 | in=2
106 | bank7:$F7B3 | d=16 | entry@move | f986 | n=205 | in=5
107 | bank7:$F744 | d=6 | entry@move | f986 | n=510 | in=3
108 | bank7:$F74A | d=-1 | entry@move | f986 | n=561 | in=5
109 | bank7:$F75B | d=15 | entry@move | f986 | n=346 | in=2
110 | bank7:$F682 | d=15 | entry@move | f986 | n=23 | in=1
111 | bank7:$F844 | d=15 | entry@move | f986 | n=26 | in=2
112 | bank7:$F68D | d=3 | entry@move | f986 | n=10 | in=3
113 | bank7:$F690 | d=16 | entry@move | f986 | n=10 | in=3
114 | bank7:$F693 | d=15 | entry@move | f986 | n=10 | in=3
115 | bank7:$F676 | d=15 | entry@move | f986 | n=23 | in=1
116 | bank7:$F7BA | d=15 | entry@move | f986 | n=23 | in=1
117 | bank7:$F7CA | d=-1 | entry@move | f986 | n=3 | in=1
118 | bank7:$F699 | d=-1 | entry@move | f986 | n=3 | in=1
119 | bank7:$F6B8 | d=-1 | entry@move | f986 | n=266 | in=2
120 | bank7:$F839 | d=-1 | entry@move | f986 | n=3 | in=1
121 | bank7:$F40A | d=15 | entry@move | f986 | n=690 | in=1
122 | bank7:$F536 | d=16 | entry@move | f986 | n=690 | in=1
123 | bank7:$F56B | d=-1 | entry@move | f986 | n=1013 | in=2
124 | bank7:$F40D | d=14 | entry@move | f986 | n=690 | in=1
125 | bank?:$9A4C | d=17 | entry@move | f986 | n=529 | in=1
126 | bank?:$9A44 | d=17 | entry@move | f986 | n=44 | in=1
127 | bank7:$E094 | d=3 | sub@move | f986 | n=690 | in=3
128 | bank7:$F416 | d=3 | entry@move | f986 | n=690 | in=1
129 | bank7:$F42D | d=-1 | entry@move | f986 | n=690 | in=1
130 | bank7:$D4BF | d=12 | sub@move | f986 | n=412 | in=1
131 | bank7:$D4CB | d=13 | sub@move | f986 | n=1236 | in=3
132 | bank7:$CC0D | d=7 | sub@move | f986 | n=412 | in=1
133 | bank7:$CF5D | d=4 | sub@move | f986 | n=22 | in=3
134 | bank7:$CF81 | d=5 | sub@move | f986 | n=22 | in=1
135 | bank7:$CFB7 | d=6 | sub@move | f986 | n=22 | in=1
136 | bank7:$CF4C | d=5 | sub@move | f986 | n=22 | in=1
137 | bank7:$CF4E | d=6 | sub@move | f986 | n=22 | in=1
138 | bank7:$E1E1 | d=7 | sub@move | f986 | n=311 | in=3
139 | bank7:$CD6D | d=7 | sub@move | f986 | n=412 | in=2
140 | bank7:$CD9C | d=8 | sub@move | f986 | n=413 | in=2
141 | bank7:$CE66 | d=9 | sub@move | f986 | n=412 | in=1
142 | bank7:$E269 | d=5 | sub@move | f986 | n=412 | in=1
143 | bank7:$E25D | d=4 | sub@move | f986 | n=412 | in=1
144 | bank7:$E37A | d=6 | sub@move | f986 | n=412 | in=1
145 | bank7:$D976 | d=7 | sub@move | f986 | n=431 | in=5
146 | bank7:$F186 | d=8 | sub@move | f986 | n=1818 | in=4
147 | bank7:$E3E5 | d=6 | sub@move | f986 | n=412 | in=1
148 | bank7:$CD92 | d=8 | sub@move | f986 | n=412 | in=1
149 | bank7:$CC98 | d=7 | sub@move | f986 | n=11 | in=1
150 | bank7:$CCB7 | d=7 | sub@move | f986 | n=316 | in=3
151 | bank7:$D7B3 | d=10 | sub@move | f986 | n=412 | in=1
152 | bank7:$D9D4 | d=9 | sub@move | f986 | n=412 | in=1
153 | bank7:$FA9D | d=12 | sub@move | f986 | n=412 | in=1
154 | bank7:$FC65 | d=12 | sub@move | f986 | n=412 | in=1
155 | bank7:$F93B | d=12 | sub@move | f986 | n=412 | in=1
156 | bank7:$F949 | d=13 | sub@move | f986 | n=2472 | in=2
157 | bank7:$FA18 | d=5 | sub@move | f986 | n=2472 | in=1
158 | bank7:$FBDD | d=12 | sub@move | f986 | n=412 | in=1
159 | bank7:$FBEC | d=13 | sub@move | f986 | n=1648 | in=2
160 | bank?:$8B87 | d=7 | sub@move | f986 | n=1648 | in=1
161 | bank7:$FE1B | d=6 | sub@move | f986 | n=412 | in=1
162 | bank7:$FE27 | d=7 | sub@move | f986 | n=5356 | in=2
163 | bank7:$F0A2 | d=11 | entry@move | f986 | n=2472 | in=1
164 | bank7:$F152 | d=8 | sub@move | f986 | n=1082 | in=1
165 | bank7:$F1EE | d=9 | sub@move | f986 | n=5718 | in=2
166 | bank7:$CE84 | d=4 | sub@move | f986 | n=1349 | in=4
167 | bank7:$F140 | d=12 | entry@move | f986 | n=935 | in=1
168 | bank7:$F1BF | d=8 | sub@move | f986 | n=935 | in=1
169 | bank7:$F1E0 | d=9 | sub@move | f986 | n=975 | in=2
170 | bank7:$F1FA | d=8 | sub@move | f986 | n=975 | in=2
171 | bank7:$F266 | d=9 | sub@move | f986 | n=1028 | in=3
172 | bank7:$E449 | d=6 | sub@move | f986 | n=1072 | in=2
173 | bank7:$F282 | d=12 | entry@move | f986 | n=935 | in=1
174 | bank7:$F0ED | d=-1 | entry@move | f986 | n=412 | in=1
175 | bank7:$F172 | d=9 | sub@move | f986 | n=452 | in=2
176 | bank7:$CE92 | d=3 | sub@move | f986 | n=412 | in=1
177 | bank7:$E0C1 | d=4 | sub@move | f986 | n=412 | in=1
178 | bank7:$E173 | d=5 | sub@move | f986 | n=824 | in=2
179 | bank7:$FAF2 | d=12 | sub@move | f986 | n=412 | in=1
180 | bank7:$FAFF | d=13 | sub@move | f986 | n=1648 | in=2
181 | bank7:$DB17 | d=12 | sub@move | f986 | n=412 | in=1
182 | bank7:$DB37 | d=12 | sub@move | f986 | n=412 | in=1
183 | bank7:$DB42 | d=13 | sub@move | f986 | n=824 | in=2
184 | bank7:$FDE3 | d=4 | sub@move | f986 | n=412 | in=1
185 | bank7:$FE05 | d=5 | sub@move | f986 | n=2060 | in=2
186 | bank7:$F3E6 | d=14 | entry@move | f987 | n=646 | in=1
187 | bank?:$9A64 | d=17 | entry@move | f987 | n=395 | in=1
188 | bank?:$9AE2 | d=18 | entry@move | f987 | n=790 | in=2
189 | bank?:$9AE9 | d=17 | entry@move | f987 | n=790 | in=2
190 | bank?:$84FE | d=3 | sub@move | f987 | n=97 | in=3
191 | bank?:$9AA8 | d=18 | entry@move | f987 | n=395 | in=1
192 | bank?:$9AAA | d=-1 | entry@move | f987 | n=395 | in=1
193 | bank?:$9AC5 | d=-1 | entry@move | f987 | n=394 | in=1
194 | bank?:$9AC6 | d=18 | entry@move | f987 | n=395 | in=1
195 | bank?:$9ADA | d=19 | entry@move | f987 | n=401 | in=2
196 | bank?:$9AE1 | d=7 | entry@move | f987 | n=401 | in=2
197 | bank?:$844B | d=3 | sub@move | f987 | n=437 | in=3
198 | bank7:$E783 | d=5 | entry@move | f987 | n=151 | in=3
199 | bank7:$E792 | d=6 | sub@move | f987 | n=470 | in=2
200 | bank7:$E8CE | d=8 | sub@move | f987 | n=919 | in=2
201 | bank7:$E90F | d=8 | sub@move | f987 | n=919 | in=2
202 | bank7:$E95F | d=9 | sub@move | f987 | n=919 | in=2
203 | bank7:$E96A | d=8 | sub@move | f987 | n=1281 | in=1
204 | bank7:$D651 | d=8 | sub@move | f987 | n=1281 | in=1
205 | bank7:$CF88 | d=6 | sub@move | f987 | n=27 | in=1
206 | bank7:$D09C | d=8 | sub@move | f987 | n=27 | in=1
207 | bank7:$CF2E | d=4 | sub@move | f987 | n=302 | in=2
208 | bank7:$F51E | d=15 | entry@move | f988 | n=323 | in=1
209 | bank7:$E4A3 | d=6 | sub@move | f989 | n=23 | in=1
210 | bank7:$E519 | d=7 | sub@move | f989 | n=20 | in=1
211 | bank7:$E668 | d=6 | sub@move | f989 | n=237 | in=1
212 | bank7:$E365 | d=6 | sub@move | f989 | n=415 | in=2
213 | bank7:$E7AD | d=7 | sub@move | f991 | n=388 | in=2
214 | bank7:$E8BE | d=8 | sub@move | f991 | n=449 | in=2
215 | bank7:$CB96 | d=3 | sub@move | f991 | n=30 | in=2
216 | bank7:$D147 | d=6 | sub@move | f991 | n=4 | in=2
217 | bank7:$CDBF | d=8 | sub@move | f992 | n=278 | in=1
218 | bank7:$E904 | d=5 | entry@move | f993 | n=319 | in=2
219 | bank7:$E98E | d=8 | sub@move | f993 | n=362 | in=1
220 | bank7:$E88B | d=7 | sub@move | f993 | n=30 | in=1
221 | bank7:$E6D2 | d=7 | sub@move | f1001 | n=222 | in=1
222 | bank7:$E54A | d=8 | sub@move | f1001 | n=360 | in=2
223 | bank?:$848C | d=6 | sub@move | f1004 | n=396 | in=3
224 | bank?:$9AD6 | d=17 | entry@move | f1004 | n=6 | in=1
225 | bank?:$9A8F | d=18 | entry@move | f1004 | n=7 | in=2
226 | bank?:$84A7 | d=3 | sub@move | f1005 | n=355 | in=3
227 | bank7:$E8F1 | d=4 | entry@move | f1005 | n=264 | in=1
228 | bank?:$9AAC | d=-1 | entry@move | f1046 | n=1 | in=1
229 | bank?:$9AB4 | d=17 | entry@move | f1046 | n=1 | in=1
230 | bank7:$E249 | d=7 | sub@move | f1077 | n=2 | in=2
231 | bank7:$F752 | d=16 | entry@move | f1097 | n=81 | in=1
232 | bank?:$81F6 | d=5 | sub@move | f1097 | n=110 | in=2
233 | bank?:$99FA | d=17 | entry@move | f1097 | n=161 | in=1
234 | bank?:$9A00 | d=7 | entry@move | f1142 | n=120 | in=1
235 | bank7:$F410 | d=3 | entry@move | f1142 | n=120 | in=1
236 | bank?:$8244 | d=3 | sub@move | f1143 | n=59 | in=1
237 | bank?:$822B | d=4 | sub@move | f1143 | n=59 | in=1
238 | bank7:$C2BE | d=4 | sub@move | f1143 | n=59 | in=1
239 | bank?:$8318 | d=3 | sub@move | f1143 | n=59 | in=1
240 | bank7:$C3D4 | d=3 | sub@move | f1143 | n=179 | in=5
241 | bank?:$816E | d=3 | sub@move | f1143 | n=52 | in=1
242 | bank7:$CF55 | d=3 | sub@move | f1151 | n=11 | in=2
243 | bank7:$E626 | d=6 | sub@move | f1183 | n=178 | in=1
244 | bank7:$E880 | d=7 | sub@move | f1184 | n=30 | in=1
245 | bank7:$F2E8 | d=13 | entry@move | f1187 | n=1 | in=1
246 | bank7:$F340 | d=6 | entry@move | f1187 | n=1 | in=1
247 | bank7:$F2D8 | d=-1 | entry@move | f1187 | n=1 | in=1
248 | bank7:$F338 | d=10 | sub@move | f1187 | n=1 | in=1
249 | bank7:$C3FB | d=3 | sub@move | f1187 | n=2 | in=2
250 | bank7:$C41D | d=4 | sub@move | f1187 | n=2 | in=1
251 | bank7:$F323 | d=4 | sub@move | f1187 | n=1 | in=1
252 | bank7:$CBDE | d=8 | sub@move | f1188 | n=1 | in=1
253 | bank7:$E457 | d=6 | sub@move | f1188 | n=3 | in=1
254 | bank7:$E7A2 | d=7 | sub@move | f1188 | n=1 | in=1
255 | bank?:$9A14 | d=-1 | entry@move | f1202 | n=2 | in=1
256 | bank7:$E6A7 | d=7 | sub@move | f1205 | n=138 | in=1
257 | bank7:$F262 | d=9 | sub@move | f1257 | n=32 | in=1
258 | bank?:$9A27 | d=8 | entry@move | f1261 | n=4 | in=1
259 | bank7:$FD8F | d=3 | sub@move | f1261 | n=40 | in=1
260 | bank7:$DC7F | d=7 | sub@move | f1261 | n=40 | in=1
261 | bank7:$DC82 | d=8 | sub@move | f1261 | n=40 | in=1
262 | bank7:$F1A7 | d=9 | sub@move | f1261 | n=40 | in=1
263 | bank?:$83F5 | d=3 | sub@move | f1270 | n=101 | in=2
264 | bank7:$E770 | d=4 | entry@move | f1270 | n=56 | in=1
```

### B.1 动态真边静态未见（机械生成原文）
```
$9A7E → $9AE2
$9A81 → $9AA8
$9A84 → $9AC6
$9AC6 → $9ADA
$9AC9 → $9AE2
$9AD0 → $9ADA
$9AD6 → $9A8F
$E789 → $E792
$E906 → $E792
$EB06 → $EB0C
$EB11 → $EB7A
$EB19 → $EB28
$EB20 → $EB4D
$EB56 → $C2C5
$EB68 → $EB85
$EB74 → $FB7B
$F0A8 → $F152
$F0CF → $F1EE
$F0DA → $CE84
$F0DF → $F140
```

## 7. 【2026-10-15 修正·P12 三件（verifier 复审 pass 后转办）】

> 本节数字**取代 §1/§3/附录 B 的「55 静态未见」**（历史主张保留，见下更正注）。

1. **对账口径修正**（§3/附录 B 的 55 虚高）：「静态边」原只比 seed 代码项——
   岛内流调用点+跳转表边同属静态 xref 证据面未入账（$E789→$E792 案：r2 已知调用点
   被误报「静态未见」）。补全对账全集（静态边 1056→**1812 对**）后：
   **动态 jsr 边静态未见 55→1**（唯一真未见 `$F438→$E094`——顺带构成
   $F438-$F43D 尾段为代码的正面证据，呼应 r2 续篇 §8 尾段待查）。
2. **P12-F2 多 bank 展开消歧**已入 call_graph merge：窗口 site 逐 bank 查
   `20 lo hi`==callee ⇒ 唯一命中收编「静态证实@bankN」（本次 7 条全落 bank1，
   与 verifier 抽核 4/4 同面）。
3. **P12-F1 类级修复（修一个实例≠修一类）**：P7-F2 只修了 decode_sweep_dyn 一处——
   **静默 0 类清扫**后共 5 处收敛为统一 `require_file`/FATAL 语义
   （call_graph load_run 双腿 / anchors --static / --anchor-meta / merge --static /
   decode_sweep --matrix / decode_sweep_dyn --static）。**8 项负测试逐点实测**：
   ```
   T1 call_graph merge --run 缺失        → FATAL: traces 前缀无任何产物文件: /nonexistent/ep.entries/.edges.jsonl
   T2 call_graph merge --repeat 第二腿缺失 → FATAL: traces 前缀无任何产物文件: /nonexistent/run2.entries/.edges.jsonl
   T3 call_graph anchors --static 显式缺失 → FATAL: anchors --static 不存在: /nonexistent/sweep.json
   T4 call_graph merge --static 缺失      → FATAL: merge --static 不存在: /nonexistent/sweep.json
   T5 call_graph --anchor-meta 显式缺失   → FATAL: --anchor-meta 不存在: /nonexistent/meta.json
   T6 decode_sweep_dyn --traces 缺失      → FATAL: traces 前缀无任何产物文件: /nonexistent/ep.*.jsonl
   T7 decode_sweep_dyn --static 缺失      → FATAL: --static 不存在: /nonexistent/sweep.json
   T8 decode_sweep --matrix 缺失          → FATAL: --matrix 不存在: /nonexistent/m.json
   ```
   教训入知识库：[gotcha-silent-zero-fix-class-not-instance](../knowledge/gotcha-silent-zero-fix-class-not-instance.md)。
4. **P12-F3 双跑命名对称化**：`--repeat` 产物一律 `<prefix>.run1.*` / `<prefix>.run2.*`
   后缀（本次已就地重排：execpath-boot.run1/run2 ✓）；旧别名目录已删，防「工件缺失」误报。
