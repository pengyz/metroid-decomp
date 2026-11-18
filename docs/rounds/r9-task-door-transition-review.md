# r9 复核：门过渡机制场景 / `door-transition` 台账升级（verdict = reject）

复核对象：`3ec344f9`（场景 + 台账升级 + lint 扩口径）、`37a9789c`（机制级负对照单测 + 对照强弱记账）。
复核员：独立子代理 `28179550-f140-4943-80bd-0a477de6e511`（证伪立场，只读，禁长跑）。

## VERDICT: reject —— `door-transition → verified` 必须撤回

### P0-1 窗口里 `SamusEnterDoor($8B13)` 根本没跑过 ⇒ f482 的 `$5A=$17` 不是「门目标房」

asm 规定：`SamusEnterDoor` 一旦命中必然留下三件签名——`DoorDelay($59)←$12`、
`DoorEntryStatus($56)|=$80`、`Samus.status←sa_Door(5)`
（`refs/m1disasm/SRC/areas_common.asm:1385-1398`、`:1410-1413`；判据 `:1322-1325`）。

实测（可复算）：
- mesen 逐帧：`data/status/mechanism_scene_door_transition.json` 的 `status` 在 f467–600 恒为 1，
  全窗取值 `{0,0x14..0x18,0xFF,1}`——**从不出现 5**。
- rust 六采点：`$0056/$0058/$0059/$004E` 全 `00`：
  `python3 -c "h=open('shots_rust/case-mechanism-door-transition_f00481_door-trigger.ram.hex').read().strip();print([h[a*2:a*2+2] for a in (0x56,0x58,0x59,0x4e)])"`
- 门挡块 `$006C/$006D` 六点全 `00`；房号查找的闸门是 `ScrollBlockOnNameTable3`
  （`refs/m1disasm/SRC/prg7_engine.asm:8043-8050`）。
- 第二次独立运行（`analysis/traces/_probe-door_f*.ram.hex`，跨一次 `$0050` 3→4 变化）同样全 0。

⇒ f482 的写点落在 `GetRoomNum/LE733`（`prg7_engine.asm:8049,8062-8095`，
`$5A←WorldMapRAM[MapY*32+MapX]`）——**滚动房号查找**，不是门过渡。
该场景实证成立的是「横向滚动 + 房号查找 + `SetupRoom` 载入链 + 邻房渲染」。

### P0-2 决定性写点那一帧不在任何观察面内，提交信息与数据矛盾

- rust 缺帧集合含 **482/483**（`compared=1168=584×2`；相位契约见
  `crates/game-core/src/frame/engine.rs:107-120`：房间载入期是 iteration span，不产行）。
- `room=$17` 只存在于 mesen 的 f482–483；因此报告里 `distinct{room:5}` 实为
  `{0x00,0xFF,0xF1,0xF2,0xF4}`——**目标房 0x17 一格都没进比较**。
- `3ec344f9` 的提交信息写「482=`$17`…两侧逐帧一致」，与数据相反。

### P1

1. `informative` 仍是弱判据：status 的 8 个取值里 7 个来自 f1–466 的标题/前奏，
   机制窗 f467–600 的 status 恒 1 ⇒ `anchor=status` 在窗内零跳变；可比 room 值里
   3 个是载入链阶段计数（`prg7_engine.asm:8673` 的 `inc RoomNumber`，`$F0` 起递增），
   而本仓已裁定「加载链中间值的帧级比较无意义」
   （`docs/findings/movie-6725m-residual-episodes.md:66-69`）。
2. verified 门槛两处互相冲突：计划 `:234` 只要求「机制级最小重现 + 0px」；
   `movie-6725m-residual-episodes.md:75-78` 另要求「≤120 帧、窗内 status ≥3 次跳变」，
   本场景 600 帧、窗内 0 次跳变 ⇒ 不满足，且门槛漂移没记账。
3. 对照记账过强：单测只钉 `physics.rs:1407-1413` 的 `$58==0` 早退，
   不能证明**场景**里那条写点来自门事件；把它叫「强对照/主证据」不成立
   （但测试本身有效：删早退后 `$56` 会被写成 `$80`、`$58` 被写成 `2`，断言必红）。
   弱对照（零输入）三采点 `$5A=00 status=00` 对「门过渡需不需要进门动作」零判别力。

### P2

1. findings `:64` 把 DoorEntryStatus 写成 `$005B`——`$5B` 是 `SpritePagePos`，
   DoorEntryStatus 是 `$56`（`constants_ram.asm:571,591`）。
2. lint 注释说「作者在 f22/f26 按 Start」——真实 inputlog 的 Start 在第 14/18 行，
   且落在 440 帧吞输入窗内 ⇒ inputlog 路径抓不到「菜单误删」形态（WARN 非阻塞，需写明）。
3. `data/status/handler_touchlist.json` 的 doc/source_run 描述陈旧（写「14 案/17 案」，
   实际 `cases_covered=19`）。
4. 观察面结构上记不了门寄存器（`tools/movie_ref/align_legs.py:82-87` 只有
   status/room/health/scrolldir；`mechanism_scene.py:43-45` 固定 `STATE_FIELDS`）。

### 数字自洽（复核通过项）

149 格 = 143 match + 6 diff；touchlist `75040` 与 sidecar 实算一致（含临时 `_probe-door` 剔除）；
6 张 rust PNG md5 互异、470/481 的 `ppu.hex` 同 sha；lint 3 条测试确实打穿（删 inputlog 分支⇒红）。

## 返工处置（见后续 commit）

1. `door-transition` **撤回 `route-only`**（走 `coverage_ledger.py --revert`，条目留 `reverts[]` 审计）。
2. 本场景证据改挂 `room-load-chain`（它才是这套证据真正支撑的条目），并保持 `route-only`：
   决定性写点 482/483 不在观察面 + 载入链中间值帧级比较已裁定无意义 ⇒ 不足以升 verified。
3. 文档按实测重写（不写「门过渡已验证」，改写成「滚动房号查找 + 载入链」）。
4. 未做（登记为后续任务）：把 `$56/$58/$59` 加进 `FIELD_ADDRS`/`STATE_FIELDS`，
   并补一个**真实**门窗口（status 出现 5 且 `$59=$12`）才能重提 `door-transition`。

## 第二轮复核（返工后）：pass-with-findings

复核员逐条核了返工，P0 全关；新 P1 是实测的：
- **已通过**：①撤回 route-only + `reverts[]` + 基线同步（verified 1 / route-only 2）②证据改挂
  `room-load-chain` 且该条目仍 route-only（不过标）③文档更正（「482 逐帧一致」「整窗逐帧相同」
  「`$005B`」三处已无残留）④反升级路径（revert 只降不升；`--update` 在降级后 NO-OP）⑤§4 对照记账
  （单测只证函数契约）⑥touchlist 头部按 19 案更正 ⑦revert 三测真打穿（14 passed）。
- **新 P1（已修，commit `27898510`）**：`_entry()` 丢字段 ⇒ **任何 load→save 都会抹掉 `reverts[]`**
  （实测再 revert 一条后前一条 1→0），与「可审计降级」承诺冲突。修法：`_entry/load` 透传 +
  `_validate_item` 校验 `REVERT_KEYS` + 新测 `test_reverts_survive_a_second_revert`。
- **新 P2（已修）**：`revert()` 用 `_baseline_doc(ledger.items, …)` 重建基线，基线里存在而台账
  缺失的 id 会被静默清除 ⇒ 增收缩守卫（`缺基线中的条目`）+ 测
  `test_revert_refuses_to_shrink_ledger`（断言消息而非仅 raise）。
- **P2（记录，未修）**：基线 `reason` 是单槽，后续写盘会覆盖本次降级说明——持久记录以
  `reverts[]` 为准，这是设计取舍。
- **P2（记录，未修）**：仍无 `promote` CLI，升级只能手改台账；`_validate_item` 不校验 evidence
  文件存在 ⇒ 降级被机械化+审计、升级仍是裸手改，通道不对称（下一步应补 `--promote`）。
- **P3（已做）**：`door-transition` 的 revert 记录加 `note`，标注「已判非门过渡、证据归属
  room-load-chain，保留仅供追溯」。
