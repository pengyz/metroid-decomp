# R9 对抗复核 verdict——7aade857（敌爆亡链全径+探针对正+两修回装）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- **verdict：pass**

## 1. 探针对正核 —— PASS（原始 trace 独立复算）

- ens_probe.lua（analysis/_gen/）+ ens_trace.txt：**WRITTEN 7479** 事件、
  `addMemoryCallback(fn,"write",0x0400,0x04FF)` 字符串 API、`st["cpu.pc"]` PC 锚
  ——与声明同构。
- **独立复算**（本复核脚本对原始 trace 重新分组）：全部 16 偏移×PC 聚类——
  **+06=$F3CC（chunkyExplosionTimer）/+09=$FB87（delay）/+0B=$F86F（health）/
  +0D=$F863（data0D）/+0E=$F437（weaponAction）/+0F=$F41B+$F539（specialAttribs/
  HP dec 同址）**——声明的六偏移映射逐项命中 ✓。
- 「位移 2=hex/dec 误读」论证：+14（dec）=**0x0E**=weaponAction ✓——hex/dec
  混读误判成立（0x14 并非映射面）。
- rust map 全对：mainloop.rs 实读 +0x0B health（:714）/+0x0D data0D（:709/:836）/
  +0x0E weaponAction（:703）/+0x0F specialAttribs（:700）/+9 delay（:646/:2097）
  ——与 trace 实证映射逐项同 ✓。

## 2. 伤害链 asm 对读（抽 3 点） —— PASS

- **F536**（EnemyReactToSamusProjectile:10861-）：isHit `and #$20` 门→冰冻臂
  （wa_IceBeam+非 miniboss+非已冻⇒data0D=$40 冻计时；metroid health=$05 特例）+
  Hurt dec 径（Lx314/316 段）——rust enemy_react_to_projectile 同构（isHit bit5/
  冰冻 data0D=$40/HP dec）✓。
- **F611**（ExplodeEnemy:11024-）：status←explode + GetEnemyTypeTimes2PlusFacing-
  Direction→EnemyDeathAnimIndex→InitEnAnimIndex + chunkyExplosionTimer ✓
  ——rust explode_enemy 同形。
- **DCF5**（EnemyCompleteDeath:5938-）+ PickupTypeTable（$DE35，:6197）——
  **ROM hexdump $DE35 = `80 81 89`** ✓（bank7 基直证）——rust pickup 表/回补同。

## 3. 回装两修连锁闭环 —— PASS

- 本轮 fresh run-all：**blocked 0**（42/67 时代blocked 面已消，48/74 时代亦
  blocked 0）——前轮「两修实装⇒blocked manifest」的 FAIL 面消失=回装后连锁解除
  实证 ✓。
- 回装件（radius 写+bit1 闩）落位与爆亡链（F40D/DE05 全径）同 commit 闭环——
  上轮撤回的「回装证据包」条件达成。

## 4. 测有效 + kill 场景 —— PASS

- `shoot_kill_full_chain`：HP=1+isHit bit5+wa=弹 ⇒ status3 爆 + chunky 计时（+06）
  ⇒ 计时走尽 ⇒ status5 拾取——**全链 mutation 锚**（任一环撤除即红）；
  `pickup_missile_restores`：+5 弹回补+槽释 ✓。
- **kill 场景 6/7 点复现**（case-killenemy-probe fresh）：game-in/at-enemy/hit-1/
  hit-2/dying/dead 六点 **0/61440 全 match**；after（f800）=221px（land-settled
  同族时序残，已知类）。点值表与声明一致。

## 5. EN_ANIM_TABLE 扩 + 烘焙一致性 —— PASS

- 64→**128B**：baker 证伪更正注在案（EnemyDeathAnimIndex max idx=0x74+序 0xFF 终符
  ⇒ ≥0x80；死 anim 走表越界 E0BC-wrap 实证——ROM $E0BC 区字节在案）；
  烘焙脚本与 sprite_data 同款截断教训修正（项计+裕量）——三发修一致性 ✓。

## 6. 必跑验证 —— PASS

- `cargo test --workspace`：**449 passed / 0 failed**（game-core **415**=413+2 ✓）。
- L4 fresh run-all：**48/74**（新增 killenemy 7 点：6 match+1 时序残；blocked 0）。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 探针对正（7479 事件独立复算六映射+hex/dec 误读论证+rust map 全对） | pass |
| 2 伤害链抽 3（F536/F611/DCF5+ROM $DE35=80 81 89） | pass |
| 3 回装闭环（blocked 0+爆亡链同 commit） | pass |
| 4 测有效（全链 mutation）+kill 6/7 点 | pass |
| 5 EN_ANIM 128B 扩+烘焙一致 | pass |
| 6 验证（449/0；gates 13/0；48/74） | pass |

**verdict = pass**。

Evidence: analysis/_gen/ens_probe.lua+ens_trace.txt（7479 事件本复核独立重分组）；
prg7_engine.asm :10861-（F536）/:11024-（F611）/:5938-（DCF5）/:6197（PickupTypeTable）；
ROM hexdump $DE35/$E0BC；mainloop.rs 槽偏移实读+tests_kill_chain；
case-killenemy-probe 矩阵行（fresh 6/1）；enemy_gfx.rs/bake_enemy_gfx.py diff；
cargo test（449/0）；gates check（PASS=13 FAIL=0）；shots_rust/matrix.md（48/74）。
