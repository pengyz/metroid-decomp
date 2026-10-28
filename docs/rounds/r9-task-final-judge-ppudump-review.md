# R9 对抗复核 verdict——29b346cb（dumpPpu harness 扩展）+ 62d162e6（终结判收口+BUG-089）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-28
- **verdict：pass**（附两处非阻塞瑕疵注记，见 §1）

## 1. 方法有效性 —— PASS（附注记）

- **采集点并联取真**：`scenario_capture.lua::dumpPpu`（29b346cb）挂于采集回调
  （shot/dumpRam 同点），f1000 采集点即取该帧时刻 PPU 总线 $0000-$2FFF+palette32B
  +scroll 3B——与截图同瞬。
- **mt=9 实证（功能自证）**：ppu_probe3 产物文件未单独归档（瑕疵注记②候选），
  但 dump 自身即功能实证——emu dump 四区（$0000-1FFF/$2000-23FF/attr/$3F00）与
  rust 侧 chr/nametable/palette 语义区逐字节同构命中，mt=9=PPU 总线成立
  （若为 CPU RAM 不可能四区同时命中）。
- **同构性**：双侧均 12323B（12KB+32B+3B）。**瑕疵注记①**：emu hex2 大写、
  rust 小写——「12323B 全对」仅在大小写归一后成立；且「全对」措辞过强：
  实际全同面=四面（CHR 8192+NT0 960+attr 64+palette 32），镜像半区（$2400+ 段）
  异（B 缓冲异房内容，预期内非主张面）、尾 3B 异（emu getState 字段回退=000000，
  porter 自注在案）。主张本身（四面全同）精确成立，措辞需收窄。
- **scroll 3B 回退面论证核**：porter「不影响定性」成立——定性仅依赖四面内容同；
  但须记残差：细滚相位因此面未验（已含于时序类 bucket，findings §5 附注已载）。

## 2. 结论核 —— PASS

- **可复现**：重跑 `l4_diff_run.py run-all` 后重比对（本复核独立脚本）：
  CHR/NT0 tiles/NT0 attr/palette 四面 **SAME**（REPRO 实测）。
- **第三可能排除（rust 渲染器瓦 fetch 逻辑错）**：查 L4 矩阵——**滚动中帧有
  0px match 点**（case-leftwalk-probe f520-790 = 左滚 80-350px 全 match），
  即 rust 渲染器横滚/细滚 fetch+调色路径已被匹配帧钉住；f420-790 静态面同钉
  非滚径。内容四面同+渲染径已验 ⇒ 残差收敛至精灵帧位/采样瞬间/帧相位——
  「时序类」定性成立（细滚相位与采样瞬间未进一步分离，属同类记账面）。

## 3. BUG-089 台账 + L4 保持 —— PASS

- BUG-089（bug-ledger.md）：现象（11267/11256px）/定性+证据（四面全同+§5 指针）/
  复现命令/影响（TOLERATED 记账，D4）/trace id（ppu.hex 路径）——五要素齐，合规。
- L4 复跑：**match 23 / diff 16（23/39 持平）**；门链五点（002 f420/620/650/700/790）
  全 match；落地三点（001 f715/725/735）全 match；f1000=**11267**、f1150=**11256**
  未动。

## 4. 必跑验证 —— PASS

- `cargo test --workspace`：**438 passed / 0 failed**（game-core 404+3+27+4）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 方法（并联取真/mt9 自证/同构） | pass（注记：hex 大小写需归一；「全对」措辞收窄为四面） |
| 2 结论（可复现+第三可能排除） | pass |
| 3 BUG-089 合规+L4 保持 | pass |
| 4 验证（438/0；gates 13/0） | pass |

**verdict = pass**（两注记不阻塞：①双侧 dumpPpu 导出 hex 大小写不一致——建议后续
统一（harness 卫生项）；②ppu_probe3 产物未归档——mt9 已由 dump 功能自证兜底）。

Evidence: analysis/traces/case-002…f01000…ppu.hex vs shots_rust/case-002…ppu.hex
（24646 hex chars，四面逐段比对脚本实测+REPRO 复跑）；scenario_capture.lua dumpPpu
（29b346cb diff）；rust_screenshot.rs 同构导出（62d162e6 diff）；bug-ledger.md BUG-089；
scroll-content-mapping.md §5；shots_rust/matrix.md（2026-10-28 复跑）；cargo test
（438/0）；gates check（PASS=13 FAIL=0）。
