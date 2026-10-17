# findings：T45-1 Statue 门控族考证（收官段考证窗 1——task-37 对表核心）

> 2026-10-17；asm-first 考证窗（m2 拼装间隙）。产出=门控语义规格（T45-1 单元的
> 拼装规格源）+task-37 对表判定+BUG 注保真面。

## 门控链（考证定谳——双 Boss 击败→双像门开→Tourian 桥）

```
miniboss 被杀（LDD75「miniboss was just killed」）
  → PowerUpMusic + MiniBossKillDelayFlag←InArea&$0F（$0108）
  → 双像闪烁位：`lsr/tay/sta KraidStatueStatus-1,y`（InArea 索引→$687B/$687C bit0）
  → +75 导弹容量（AddToMaxMissiles）
  → （时序后）UpdateStatue_StartRaising（DA7C）→ 像升起（$687B/$687C **bit7=up**）
  → UpdateAllStatues_Bridge（DADA）门判：
      ① StatuesBridgeIsSpawned（$54）bit7=已生成→出
      ② DoorEntryStatus（$56）≠0→出（在门内）
      ③ **KraidStatueStatus AND RidleyStatueStatus bpl→出**（双像必须同 bit7=双 Boss 击败）
      → $54 置位 + **8×TileBlasts 生成**（routine=3（待重生）/delay=y*2/type=4（generic
        shot block））=Tourian 桥瓦面
```

## task-37 对表判定（爆破瓦降级真实解锁径）

**真实解锁径=Statue 瓦爆面**：桥=8 个 TileBlasts 槽（爆破瓦系统）生成——降级面
（爆破瓦销毁行为 L3 降级）不影响门判逻辑（③ 双 AND 门=纯 RAM 面）；桥生成=瓦爆
域件（TileBlasts.0.routine/delay/type 写面）。**门判可与瓦爆域解耦考证**：
①②③ 门判=RAM 位语义（无降级依赖）；桥生成=瓦爆域（降级件随件）。
⇒ 降级后径可达性判据=**双像 bit7 面可置位∧桥旗 $54 可置位**（门判自证）——瓦爆
表现层降级不阻径。

## 地址账（考证自证）

| 符号 | 址 | 证据 |
|---|---|---|
| KraidStatueStatus | $687B | constants_ram:1212（「bit 0 set, the statues blink」） |
| RidleyStatueStatus | $687C | constants_ram:1213（「bit 7 set, statues are up」） |
| StatuesBridgeIsSpawned | $54 | constants_ram:547（「0=Bridge is not spawned…」） |
| InArea | $74 | constants_ram:605（区索引：$10/$00=Brinstar…$12=Kraid hideout） |
| MiniBossKillDelayFlag | $0108 | constants_ram:809（Kraid/Ridley 杀后延迟面） |
| Statue 对象槽 | Statue-Objects | UpdateAllStatues `lda #Statue-Objects/sta PageIndex`（槽位偏移随 Object 布局考证） |

## asm 内容锚（考证主体）

- `UpdateAllStatues: ; 07:D9D4`（状态机 0→1→2→3：lowered 瓦面（y=0/1）→raised 瓦面
  （y=2/3 按双像 bit7）→UpdateStatue×2→Bridge）
- `UpdateStatue: ; 07:DA1A`（Raise+StartRaising+animFrame 表）
- `UpdateAllStatues_Bridge: ; 07:DADA`（门判①②③+8 瓦爆生成环——`ldx #(8-1)*
  _sizeof_TileBlasts.0/ldy #$08`）
- `LDD75`（miniboss 杀手：`lsr/tay/sta KraidStatueStatus-1,y`——InArea 索引置闪烁位）

## BUG 注保真面（D4——随 T45-2 落）

- ChooseEnding SamusAge+3 漏检（前报在案）；本考证无新增 BUG 注。

## 待随件考证（T45-1 单元随补）

1. Statue 对象槽偏移（Statue-Objects 的具体值——Object 布局面）。
2. UpdateStatueBGTiles（DAB0）瓦面写语义（named 表 DAD2/DAD6——瓦爆域对表）。
3. UpdateStatue_StartRaising（DA7C）触发时序（像闪→升起的延迟/条件）。
4. TileBlasts 结构（_sizeof_TileBlasts.0 字段面——爆破瓦域 port2/协同面）。
