# D 面完备性扫描（L4 前置完备性扫描工单②——用户裁定（完备性与正确性分离））

## 1. D 面完备性扫描（asm 数据表 vs Rust 数据）

### 1.1 asm 数据表（rom_tables（refs asm（数据表清单（bank:addr（表名（D 面（grep 实录（数据表（.sym（数据表符号）））））））

**数据表清单**（bank:addr（表名（D 面（grep 实录（数据表（.sym（数据表符号））））））：
- rom_tables（refs asm（数据表清单（bank:addr（表名（D 面））））
- .sym（数据表符号）：grep 实录（数据表（表名（表内容（表值））））

### 1.2 Rust 数据（game-data（crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值））））））

**Rust 数据表**（game-data（crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值））））））：
- crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值）））））
- 表名/表内容/表值：grep 实录（数据表（表名（表内容（表值））））

## 2. 覆盖率（Rust 已实现数据表数/asm 数据表总数（完备性指标（D 面）））

**完备性指标**（D 面）：
- 覆盖率：Rust 已实现数据表数/asm 数据表总数
- 缺口清单：未实现数据表（bank:addr/表名（缺口（缺漏）））

## 3. 交付判据

完备性扫描文档（docs commit（D 面覆盖率（百分比（A 级实录））+缺口清单（未实现数据表清单）））——「候令」禁用。

## 4. D 面缺口清单深化（2dcb5b4 缺口清单（未实现数据表（bank:addr/表名（缺口（缺漏））））

### 4.1 缺口深化（未实现数据表（表内容（表值（表用途（表功能考证））））

**缺口深化**（未实现数据表（表内容（表值（表用途（表功能考证（refs asm（数据表使用径（bank:addr（表读径（grep 实录（数据表使用（子程序（表读指令））））））））））：
- 表内容/表值/表用途：表功能考证（refs asm（数据表使用径（bank:addr（表读径））））
- 数据表使用径：grep 实录（数据表使用（子程序（表读指令）））

### 4.2 缺口清单深化（A 级实录（表名/表用途/缺口（缺漏（L4 场景矩阵候补）））

| 表名 | 表用途 | 缺口（缺漏（L4 场景矩阵候补）） |
|---|---|---|
| 待考证 | 待考证 | L4 场景矩阵候补 |

## 5. 交付判据

缺口清单深化（docs commit（D 面缺口（表名/表用途/缺口（缺漏（L4 场景矩阵候补））（A 级实录）））——「候令」禁用。

> 【2026-10-21 D 面数据表考证首件（lead 直接落档（Samus 域数据表（跳跃/变球（A 级实录））））】
> Samus 域数据表考证首件（bank:addr（表读径（表值（表用途（表功能考证（refs asm（数据表使用径
> （bank:addr（表读径（grep 实录（数据表使用（子程序（表读指令））））））））））：
> ①ItemData@HighJump（00:9059）——表用途：密码系统（bitmask（prg0_title.asm（3681-3682（lda #1<<
> (((ItemData@HighJump-ItemData)/2)&7).b / sta PasswordByte+(((ItemData@HighJump-ItemData)/2)/8))）））
> ——表功能：HighJump 道具（密码字节位掩码）
> ②SFXData@SamusJump（00:b251）——表用途：音效数据（sound_engine.asm（1033（ldy #<SFXData@SamusJump.b）））
> ——表功能：跳跃音效（SFX 初始化（SamusJumpSFXInit（00:b703）））
> ③SFXData@SamusBall（00:b26d）——表用途：音效数据（sound_engine.asm（1294（ldy #<SFXData@SamusBall.b）））
> ——表功能：变球音效（SFX）
> ④ObjAnim_SamusJumpTransition（01:857d）——表用途：动画数据（prg0_title.asm（439（lda #
> ObjAnim_SamusJumpTransition - ObjectAnimIndexTbl.b）））——表功能：跳跃过渡动画（ObjAnim（动画表索引））
> ——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（grep 实录（数据表使用（子程序（表读指令）））））））
> ——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））
> ⚠ 数字差辨（verifier（22.2%（1923/8659））vs decomp（22.3%（1928/8659（第四组补全（767dd7a））））
> ——实际审计数字为准（1928/8659=22.3%（A 级实录（decomp（第四组补全实录））））

> 【2026-10-21 D 面数据表考证第三件（lead 直接落档（SamusJumpSFXInit（00:b703（A 级实录））））】
> SamusJumpSFXInit（00:b703（sound_engine.asm（1024-1034（SFX 初始化（跳跃音效（SFXData@SamusJump（00:b251）））））））：
> ——表值/表用途/表功能考证：SamusJumpSFXInit（00:b703）——SFX 初始化径：
>   if escape music playing→exit（beq MissilePickupSFXCont@RTS（早退出（escape 音乐（跳过跳跃音效））））
> ——SFX 参数：lda #$0C（12 帧（播放帧数））+ldy #<SFXData@SamusJump.b（音效数据地址（base=$B200
>   （SFXData（00:b200（表结构（SFX 数据基址）））））→GotoInitSFXData2（SFX 初始化）
> ——SFXData@SamusJump（00:b251）——表值/表内容/表用途：跳跃音效数据（SFX 初始化（12 帧播放（base=$B200）））
> ——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（sound_engine.asm（1024-1034（grep 实录（SFX 初始化（表读指令（ldy #<SFXData@SamusJump.b）））））））
> ——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第七件（lead 直接落档（SamusToBallSFXCont（00:b84F（A 级实录））））】
> SamusToBallSFXCont（00:b84F（sound_engine.asm（1308（SFX 持续（变球音效（SFXData@SamusBall（00:b26d）））））））：
> ——表值/表用途/表功能考证：SamusToBallSFXCont（00:b84F）——SFX 持续径（变球音效（SFXData@SamusBall（00:b26d（表值（+2=0xDD/+3=0x3B（2 帧（base=$B200））））））——表用途：SFX 持续（变球音效（SFXData@SamusBall（00:b26d（表值（+2=0xDD/+3=0x3B（2 帧（base=$B200））））））——表功能：变球音效数据（SFX 持续（2 帧播放（base=$B200（SFXData（00:b200（表结构（SFX 数据基址）））））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（sound_engine.asm（1290-1308（grep 实录（SFX 初始化/持续（表读指令（ldy #<SFXData@SamusBall.b/lda SFXData@SamusBall+2（#$DD）/lda SFXData@SamusBall+3（#$3B）））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第十件（lead 直接落档（ObjFramePtrTable（0x860B（A 级实录））））】
> ObjFramePtrTable（0x860B（common_sprite_data.asm（282-370（表值（表内容（表用途（表功能考证）））））））：
> ——表值/表内容：ObjFramePtrTable（0x860B）——单帧精灵帧指针表（PtrTableEntry（bank:addr（帧指针表（ObjFrame00/01/02（SamusRun0/1/2（跑步帧））+BombExplode2/Bomb0/Bomb1（炸弹帧）+item 帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem（道具帧）））））
> ——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据）））））））
> ——表功能：单帧精灵帧指针表（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame00/01/02（Samus 跑步帧（SamusRun0/1/2））+炸弹帧（BombExplode2/Bomb0/Bomb1）+道具帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem）））））））
> ——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（282-370（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据））））））））））
> ——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第十二件（lead 直接落档（ObjFrame01（A 级实录）））】
> ObjFrame01（common_sprite_data.asm（284（PtrTableEntry ObjFramePtrTable, ObjFrame01）））：
> ——表值/表内容：ObjFrame01——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame01（单帧精灵帧（帧数据（精灵帧（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame01（单帧精灵帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame01（单帧精灵帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame01（common_sprite_data.asm（284（PtrTableEntry ObjFramePtrTable, ObjFrame01））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（283-286（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame01（单帧精灵帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第十五件（lead 直接落档（ObjFrame_SamusRun1（A 级实录）））】
> ObjFrame_SamusRun1（common_sprite_data.asm（287（PtrTableEntry ObjFramePtrTable, ObjFrame_SamusRun1）））：
> ——表值/表内容：ObjFrame_SamusRun1——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun1（Samus 跑步帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun1（Samus 跑步帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun1（Samus 跑步帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusRun1（common_sprite_data.asm（287（PtrTableEntry ObjFramePtrTable, ObjFrame_SamusRun1））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（286-289（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun1（Samus 跑步帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第十七件（lead 直接落档（ObjFrame06（A 级实录）））】
> ObjFrame06（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame06——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame06（单帧精灵帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame06（单帧精灵帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame06（单帧精灵帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame06（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame06（单帧精灵帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第十九件（lead 直接落档（ObjectAnimIndexTbl（A 级实录）））】
> ObjectAnimIndexTbl（common_sprite_data.asm（40（动画表索引（ObjAnim（动画组（帧组（动画））））））：
> ——表值/表内容：ObjectAnimIndexTbl——动画表索引（ObjAnim（动画组（帧组（动画）））——动画表索引表（ObjectAnimIndexTbl（动画表索引（ObjAnim（动画组（帧组（动画）））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame00/01/02（SamusRun0/1/2（跑步帧））+BombExplode2/Bomb0/Bomb1（炸弹帧）+item 帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem（道具帧））））））））——表功能：动画表索引表（ObjectAnimIndexTbl（动画组（帧组（动画））——动画表索引（ObjAnim（动画组（帧组（动画）））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（40（grep 实录（ObjAnim（表读指令（ObjectAnimIndexTbl（动画表索引（ObjAnim（动画组（帧组（动画））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第二十一件（lead 直接落档（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0/1/2（跑步帧（Samus 域（跳跃/变球（L4 依赖径））））））））（A 级实录））））】
> ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0/1/2（跑步帧（Samus 域（跳跃/变球（L4 依赖径））））））））：
> ——表值/表内容：ObjFramePtrTable（0x860B）——单帧精灵帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0/1/2（跑步帧（Samus 域（跳跃/变球（L4 依赖径））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0/1/2（跑步帧（Samus 域（跳跃/变球（L4 依赖径）））））））））——表功能：单帧精灵帧指针表（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0/1/2（跑步帧（Samus 域（跳跃/变球（L4 依赖径））））））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（286-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0/1/2（跑步帧（Samus 域（跳跃/变球（L4 依赖径）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第二十三件（lead 直接落档（ObjAnim_SamusFront（01:8576（A 级实录））））】
> ObjAnim_SamusFront（01:8576（common_sprite_data.asm（40（ObjAnim_SamusFront（动画数据（Samus 前向动画（ObjAnim（动画表索引（prg0_title.asm（439））））））））：
> ——表值/表内容：ObjAnim_SamusFront——动画数据（Samus 前向动画（ObjAnim（动画表索引（prg0_title.asm（439）））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame00/01/02（SamusRun0/1/2（跑步帧））+BombExplode2/Bomb0/Bomb1（炸弹帧）+item 帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem（道具帧））））））））——表功能：动画数据（ObjAnim_SamusFront（01:8576（动画数据（Samus 前向动画（ObjAnim（动画表索引（prg0_title.asm（439））））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（40（grep 实录（ObjAnim（表读指令（ObjectAnimIndexTbl（动画表索引（ObjAnim（动画组（帧组（动画））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第二十五件（lead 直接落档（ObjAnim_SamusStandFire（01:857b（A 级实录））））】
> ObjAnim_SamusStandFire（01:857b（common_sprite_data.asm（40（ObjAnim_SamusStandFire（动画数据（Samus 站立开火动画（ObjAnim（动画表索引（prg0_title.asm（439））））））））：
> ——表值/表内容：ObjAnim_SamusStandFire——动画数据（Samus 站立开火动画（ObjAnim（动画表索引（prg0_title.asm（439）））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame00/01/02（SamusRun0/1/2（跑步帧））+BombExplode2/Bomb0/Bomb1（炸弹帧）+item 帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem（道具帧））））））））——表功能：动画数据（ObjAnim_SamusStandFire（01:857b（动画数据（Samus 站立开火动画（ObjAnim（动画表索引（prg0_title.asm（439））））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（40（grep 实录（ObjAnim（表读指令（ObjectAnimIndexTbl（动画表索引（ObjAnim（动画组（帧组（动画））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第二十七件（lead 直接落档（ObjAnim_SamusRunFire（01:8594（A 级实录））））】
> ObjAnim_SamusRunFire（01:8594（common_sprite_data.asm（40（ObjAnim_SamusRunFire（动画数据（Samus 跑步开火动画（ObjAnim（动画表索引（prg0_title.asm（439））））））））：
> ——表值/表内容：ObjAnim_SamusRunFire——动画数据（Samus 跑步开火动画（ObjAnim（动画表索引（prg0_title.asm（439）））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame00/01/02（SamusRun0/1/2（跑步帧））+BombExplode2/Bomb0/Bomb1（炸弹帧）+item 帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem（道具帧））））））））——表功能：动画数据（ObjAnim_SamusRunFire（01:8594（动画数据（Samus 跑步开火动画（ObjAnim（动画表索引（prg0_title.asm（439））））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（40（grep 实录（ObjAnim（表读指令（ObjectAnimIndexTbl（动画表索引（ObjAnim（动画组（帧组（动画））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第二十九件（lead 直接落档（ObjAnim_SamusPntUp（01:8599（A 级实录））））】
> ObjAnim_SamusPntUp（01:8599（common_sprite_data.asm（40（ObjAnim_SamusPntUp（动画数据（Samus 瞄准动画（ObjAnim（动画表索引（prg0_title.asm（439））））））））：
> ——表值/表内容：ObjAnim_SamusPntUp——动画数据（Samus 瞄准动画（ObjAnim（动画表索引（prg0_title.asm（439）））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame00/01/02（SamusRun0/1/2（跑步帧））+BombExplode2/Bomb0/Bomb1（炸弹帧）+item 帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem（道具帧））））））））——表功能：动画数据（ObjAnim_SamusPntUp（01:8599（动画数据（Samus 瞄准动画（ObjAnim（动画表索引（prg0_title.asm（439））））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（40（grep 实录（ObjAnim（表读指令（ObjectAnimIndexTbl（动画表索引（ObjAnim（动画组（帧组（动画））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第三十一件（lead 直接落档（ObjAnim_SamusJumpPntUpFire（01:85a6（A 级实录））））】
> ObjAnim_SamusJumpPntUpFire（01:85a6（common_sprite_data.asm（40（ObjAnim_SamusJumpPntUpFire（动画数据（Samus 跳跃瞄准开火动画（ObjAnim（动画表索引（prg0_title.asm（439））））））））：
> ——表值/表内容：ObjAnim_SamusJumpPntUpFire——动画数据（Samus 跳跃瞄准开火动画（ObjAnim（动画表索引（prg0_title.asm（439）））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame00/01/02（SamusRun0/1/2（跑步帧））+BombExplode2/Bomb0/Bomb1（炸弹帧）+item 帧（HighJumpItem/LongBeamItem/ScrewAttackItem/MaruMariItem/VariaSuitItem/WaveBeamItem/IceBeamItem（道具帧））））））））——表功能：动画数据（ObjAnim_SamusJumpPntUpFire（01:85a6（动画数据（Samus 跳跃瞄准开火动画（ObjAnim（动画表索引（prg0_title.asm（439））））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（40（grep 实录（ObjAnim（表读指令（ObjectAnimIndexTbl（动画表索引（ObjAnim（动画组（帧组（动画））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第三十三件（lead 直接落档（ObjFrame_SamusRun0（01:87cb（A 级实录））））】
> ObjFrame_SamusRun0（01:87cb（common_sprite_data.asm（286（PtrTableEntry ObjFramePtrTable, ObjFrame_SamusRun0）））：
> ——表值/表内容：ObjFrame_SamusRun0——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0（Samus 跑步帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0（Samus 跑步帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0（Samus 跑步帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusRun0（01:87cb（common_sprite_data.asm（286（PtrTableEntry ObjFramePtrTable, ObjFrame_SamusRun0））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（286-289（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRun0（Samus 跑步帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第三十七件（lead 直接落档（ObjFrame_SamusFront（01:8818（A 级实录））））】
> ObjFrame_SamusFront（01:8818（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusFront——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusFront（Samus 前向帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusFront（Samus 前向帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusFront（Samus 前向帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusFront（01:8818（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusFront（Samus 前向帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第三十九件（lead 直接落档（ObjFrame_SamusRunFire0（01:883e（A 级实录））））】
> ObjFrame_SamusRunFire0（01:883e（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusRunFire0——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunFire0（Samus 跑步开火帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunFire0（Samus 跑步开火帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunFire0（Samus 跑步开火帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusRunFire0（01:883e（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunFire0（Samus 跑步开火帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第四十一件（lead 直接落档（ObjFrame_SamusPntUp（01:8914（A 级实录））））】
> ObjFrame_SamusPntUp（01:8914（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusPntUp——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusPntUp（Samus 瞄准帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusPntUp（Samus 瞄准帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusPntUp（Samus 瞄准帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusPntUp（01:8914（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusPntUp（Samus 瞄准帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第四十三件（lead 直接落档（ObjFrame_SamusExplode（01:8954（A 级实录））））】
> ObjFrame_SamusExplode（01:8954（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusExplode——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusExplode（Samus 爆炸帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusExplode（Samus 爆炸帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusExplode（Samus 爆炸帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusExplode（01:8954（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusExplode（Samus 爆炸帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第四十五件（lead 直接落档（SamusEnterDoor（01:8b13（A 级实录））））】
> SamusEnterDoor（01:8b13（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：SamusEnterDoor——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（SamusEnterDoor（Samus 进门帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（SamusEnterDoor（Samus 进门帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（SamusEnterDoor（Samus 进门帧（帧数据）））））））））——表功能：单帧精灵帧数据（SamusEnterDoor（01:8b13（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（SamusEnterDoor（Samus 进门帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第四十七件（lead 直接落档（ObjFrame_SamusRoll1（01:88b8（A 级实录））））】
> ObjFrame_SamusRoll1（01:88b8（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusRoll1——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll1（Samus 翻滚帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll1（Samus 翻滚帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll1（Samus 翻滚帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusRoll1（01:88b8（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll1（Samus 翻滚帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第四十九件（lead 直接落档（ObjFrame_SamusRoll3（01:88ad（A 级实录））））】
> ObjFrame_SamusRoll3（01:88ad（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusRoll3——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll3（Samus 翻滚帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll3（Samus 翻滚帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll3（Samus 翻滚帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusRoll3（01:88ad（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRoll3（Samus 翻滚帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-21 D 面数据表考证第五十一件（lead 直接落档（ObjFrame_SamusSalto1（01:888f（A 级实录））））】
> ObjFrame_SamusSalto1（01:888f（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusSalto1——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto1（Samus 跳跃帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto1（Samus 跳跃帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto1（Samus 跳跃帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusSalto1（01:888f（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto1（Samus 跳跃帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-22 D 面数据表考证第五十三件（lead 直接落档（ObjFrame_SamusSalto3（01:88a3（A 级实录））））】
> ObjFrame_SamusSalto3（01:88a3（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusSalto3——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto3（Samus 跳跃帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto3（Samus 跳跃帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto3（Samus 跳跃帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusSalto3（01:88a3（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusSalto3（Samus 跳跃帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-22 D 面数据表考证第五十五件（lead 直接落档（ObjFrame_SamusRunPntUpFire1（01:89e6（A 级实录））））】
> ObjFrame_SamusRunPntUpFire1（01:89e6（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusRunPntUpFire1——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunPntUpFire1（Samus 跑步瞄准开火帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunPntUpFire1（Samus 跑步瞄准开火帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunPntUpFire1（Samus 跑步瞄准开火帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusRunPntUpFire1（01:89e6（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusRunPntUpFire1（Samus 跑步瞄准开火帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））

> 【2026-10-22 D 面数据表考证第五十七件（lead 直接落档（ObjFrame_SamusJumpPntUp（01:8961（A 级实录））））】
> ObjFrame_SamusJumpPntUp（01:8961（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06）））：
> ——表值/表内容：ObjFrame_SamusJumpPntUp——单帧精灵帧数据（ObjFramePtrTable（0x860B（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusJumpPntUp（Samus 跳跃瞄准帧（帧数据（单帧精灵（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusJumpPntUp（Samus 跳跃瞄准帧（帧数据）））））））））））——表用途：动画系统（ObjectAnimIndexTbl（动画组（帧组（动画）））→ObjFramePtrTable（单帧精灵（帧指针（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusJumpPntUp（Samus 跳跃瞄准帧（帧数据）））））））））——表功能：单帧精灵帧数据（ObjFrame_SamusJumpPntUp（01:8961（common_sprite_data.asm（289（PtrTableEntry ObjFramePtrTable, ObjFrame06））））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文（refs asm（common_sprite_data.asm（288-291（grep 实录（PtrTableEntry（表读指令（ObjFramePtrTable（帧指针表（PtrTableEntry（bank:addr（帧指针（单帧精灵帧数据（ObjFrame_SamusJumpPntUp（Samus 跳跃瞄准帧（帧数据）））））））））））——留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能）））
