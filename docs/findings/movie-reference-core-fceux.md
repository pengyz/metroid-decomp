# 录像参照核心：自建 FCEUX 2.2.2 与 Mesen2 的轨迹对拍（2026-11-18）

## 结论（先说数）

- **100% 录像（Lord Tom 33:25.70，.fm2 / author core emuVersion 9816）的死亡不是路线本意**：
  在自建 FCEUX 2.2.2 下，Samus 在 f4300 仍存活（hp=$0140、status=Run），
  而 Mesen2 腿在 f3890 判死（GAME OVER → PASS WORD）。
- **两核心首分歧 ≈ Mesen2 f1759 / FCEUX f1761**（对齐偏移 +2）：
  Mesen2 处血量 $02_32（掉血），FCEUX 侧保持 $03_00，此后持续不再重合。
  该点之前两核心血量/状态逐帧相同。
- 因此按"作者核心"口径，100% 录像可用段只到 **f1758**；
  f1–f3889 的 0px 仍然成立的是**移植保真**（rust == Mesen2），不是**路线保真**。

## 自建 FCEUX（发行版 2.5.0 在本机启动即 abort，必须自建）

```
# 源码：GitHub tag 快照 fceux-2.2.3 的 tags/fceux-2.2.2（SourceForge 直链返回 HTML）
python3 -m lib2to3 -w -n $(find . -name SConstruct -o -name SConscript)   # SCons 4 需 py3 语法
sed -i '117s/> 0)/!= NULL)/' fceux-server/server.cpp                        # 指针与 0 比较
sed -i 's/) throw(int);/);/' fceux-server/server.cpp                       # C++17 禁动态异常规格
scons -j$(nproc) GTK=0                                                     # 无 GTK：SDL-only
```

产物 `src/fceux`（7.5 MB）。运行：

```
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
  src/fceux --opengl 0 --sound 0 --loadlua tools/movie_ref/fceux_ref.lua \
            --playmov <movie>.fm2 <rom>.nes
```

## 取证边界

- 本机可得的最近核心是 **2.2.2**，录像标注 **9816（2.2.3）**；两者同一代，但不保证 lag 行为逐帧相同。
- 因此上面的 f1759 是"**与 2.2.2 的首分歧**"，不是与 9816 的首分歧；要精确到 9816 需拿到 2.2.3 源码（SourceForge 直链被判 HTML、GitHub tag 只到 2.2.2）。
- 帧号对齐用的是 hp 序列的最佳匹配（偏移 +2）；下一步应把 `$002D FrameCount` 与 `$005A RoomNumber` 也记进 FCEUX 日志，用 FC 做机械对齐。

## 下一步

1. FCEUX 日志补 `$002D/$005A/$030D/$030E/$002E/$002F`（RNG），用 FC 机械对齐后重算首分歧帧。
2. 在 f1700–1760 二分，定位"两核心从哪一帧起状态不同"，看是不是 RNG（`$2E/$2F`）先岔——若是，属核心时序差异，该点即 `usable_until`。
3. 把 `usable_until` 写进 `data/status/movie_ledger.json` 的 `reference_core` 段，并注明"移植保真"与"路线保真"是两件事。
