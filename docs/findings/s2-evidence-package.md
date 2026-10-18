# findings：S2 批产取证打包（共享引擎第二批——同 S1 模板）

> 状态：**S2 证据预备仓就位**（2026-10-18）。S2=共享引擎簇 31-60（规模降序）。
> 标签消解=m1disasm 名标签 737 个补源（nmikstas 壳+同注双源）；跨区=area 真源
> （m1disasm prg0-6+子目录——nmikstas Bank0X=include 壳无调用行）。

## 1. S2 簇清单（30 簇/93 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S2-30 | 4 | $F6E2; $F711; $F7B3 AndEnData05 `and $0405,x`; $F83B |
| S2-31 | 4 | $F702; $F735; $F744 OrEnData05 `ora $0405,x`; $F761 |
| S2-32 | 4 | $F97C LF97C `lda #$01`; $FA15; $FA7A; $FA9A |
| S2-33 | 3 | $CF2E LCF2E `LDA SamusHit`; $D018; $D133 |
| S2-34 | 3 | $D101; $D630; $E8BE StoreObjectPositionToTemp `lda ObjectHi,x` |
| S2-35 | 3 | $D120; $D1A9; $E1E1 BitScan `stx $0E` |
| S2-36 | 3 | $D3C8; $D900; $D92C StartMusic `lda ElevatorStatus` |
| S2-37 | 3 | $CDFA CheckHealthStatus `lda SamusHit`; $D406; $D411 |
| S2-38 | 3 | $CD92 SetmirrorCntrlBit `LDA SamusDir`; $D414; $D483 |
| S2-39 | 3 | $D43D; $D89D; $E519 |
| S2-40 | 3 | $D450; $D46E; $D47E SamusElevator `lda FrameCount` |
| S2-41 | 3 | $D45D; $D897; $E4F1 |
| S2-42 | 3 | $8296; $D560; $F9BE |
| S2-43 | 3 | $832F; $D56A; $F9C8 |
| S2-44 | 3 | $D61B; $D651 CheckBlastTile `ldy InArea`; $D770 |
| S2-45 | 3 | $D2FA SetProjectileAnim `sta AnimResetIndex,x`; $D660; $D685 |
| S2-46 | 3 | $D66B; $D68E; $D6A4 |
| S2-47 | 3 | $D648; $D847; $D85E |
| S2-48 | 3 | $EB8C; $EB92 SpawnDoorRoutine `iny`; $EEF4 DoorHandler `jsr LEB92` |
| S2-49 | 3 | $EC04; $EC09 SpawnElevatorRoutine `lda ElevatorStatus`; $EEA1 ElevatorHandler `jsr LEC09` |
| S2-50 | 3 | $EEC6 `lda #$01`; $EEEB; $EEF1 |
| S2-51 | 3 | $F0E5; $F136; $F1F4 |
| S2-52 | 3 | $F133; $F311 SamusHurt_F311 `bcs Exit22`; $FEC2 |
| S2-53 | 3 | $CE92 SubtractHealth `lda HealthLoChange`; $F13D; $FEC9 |
| S2-54 | 3 | $CD9C IsScrewAttackActive `SEC`; $F287; $F2F4 |
| S2-55 | 3 | $C2C6 Amul8 `ASL`; $F335; $FF4E |
| S2-56 | 3 | $F3CF; $F401; $F6B9 UpdateEnemy_ForceSpeedTowardsSamus `lda #$00` |
| S2-57 | 3 | $F3D2; $F404; $F75B UpdateEnemy_EnData05DistanceToSamusThreshold `lda #$E7` |
| S2-58 | 3 | $F416 `ldx PageIndex`; $F480; $F4EB |
| S2-59 | 3 | $F50B; $F515 LF515 `sta $040C,x`; $F554 |

## 1.5 消化口径（F13 钉死——A 区 v3 法同用）

**标签口径三级**：①名注优先（nmikstas Name:/m1disasm 注释锚）②L 行尾注次之
（nmikstas 注释语义）③指令头兜底（操作码直读）。

## 2. asm 锚预消化

- m1disasm 名标签补源后消解率显著提升（737 bank7 名标签可反查）；
- S2 簇规模小于 S1（93 员/30 簇=均 3 员——辅助例/短例族）；
- 定位锚=m1disasm 注释 `Name: ; 07:XXXX` 双源（nmikstas L 标签+名注）。

## 3. 跨区依赖表（S2×area——遗留②收）

| 目标 | 名 | 调用方 |
|---|---|---|
| $C2C6 | Amul8 | prg0_title.asm |

- 口径注：m1disasm 名调用匹配（jsr|jmp NAME→bank7 地址反查）——跨区边显式化
  方法定型（S3-S7 批同法）；1 目标=辅助例少被跨区调（合理）。

## 4. Table19↔TileBlastAnim 逐项对表（遗留③收）

> 【2026-10-18 更正·复审 F8 连锁】本节 InArea==$11 特例注更正：写值应为 **2**
> （非 4——A9 04 尾接共享 LSR=4>>1=2；条件双臂 InArea==$11 且瓦值==$76）——
> Table19 索引映射[2]=0x20→项8（原「Table19[4]」注随正）；重生态瓦值=项第 3 字节
> 结论不变。红门「门开」链引用限制见 s1-evidence-package.md F9 阻断注。

**定谳**：Table19[10]=位移索引（字节偏移）→TileBlastAnim 10 项×4B：
```
项 = [动画帧A, 动画帧B, 重生态瓦值, $FE 终止]
索引映射（TileType→项）：[6,7,8,0,1,2,3,4,9,5]
示例：Table19[0]=0x18→项6=[07,06,09,FE]；Table19[3]=0x00→项0=[06,07,00,FE]
```
- 重生态瓦值=第 3 字节（$00=清空开洞——红门轰开后的门洞态！）；
- InArea==$11 特例（IsBlastTile）=TileType 4→Table19[4]=0x04→项1=[07,06,01,FE]
  ——重生态 $01（非清空=特殊瓦形态）。

## 5. 遗留（滚动收口）

1. 标签细化：m1disasm 名源已接入（遗留①以双源法收）；残余 ?=区内中间标签随移植细化；
2. 跨区表：方法定型（§3）——S3-S7 同法随批出；
3. Table19↔Anim：**已收**（§4）。
