# D 面数据表考证第二十件（L4 就绪批微工单——Samus 域数据表（跳跃/变球））

## 1. D 面数据表考证第二十件（ObjAnim_SamusJumpPntUpFire（01:857d（prg7_engine.asm（3438））））

### 1.1 refs asm 考证（bank:addr（表读径（表值（表内容（表用途（表功能考证）））））

**bank:addr**（表读径（表值（表内容（表用途（表功能考证（refs asm（数据表使用径（bank:addr（表读径（grep 实录（数据表使用（子程序（表读指令））））））））））：
- ObjAnim_SamusJumpPntUpFire（01:857d）：动画数据（跳跃向上射击动画（ObjAnim（动画表索引）））
- prg7_engine.asm（3438）：ObjAnim_SamusJumpPntUpFire（-1 to get ObjAnim_SamusJumpTransition and ObjAnim_SamusJumpPntUpFire respectively）

### 1.2 refs 引用必写（bank:addr 引证（表读径实证（A 级原文）））

**refs 引用必写**（bank:addr 引证（表读径实证（A 级原文）））：
- bank:addr 引证（表读径实证（A 级原文（refs asm（bank:addr（表读指令实证）））））
- prg7_engine.asm（3438）：ObjAnim_SamusJumpPntUpFire（-1 to get ObjAnim_SamusJumpTransition and ObjAnim_SamusJumpPntUpFire respectively）

## 2. 考证文档（表名/表值/表用途/表功能（A 级实录））

| 表名 | 表值 | 表用途 | 表功能 |
|---|---|---|---|
| ObjAnim_SamusJumpPntUpFire | 01:857d | 动画数据（跳跃向上射击动画（ObjAnim（动画表索引））） | 表功能考证（refs asm（数据表使用径）） |

## 3. 更新留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能））））

**追溯径**（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能））：
- refs 引用（bank:addr）：bank:addr 引证（表读径实证（A 级原文））
- 考证成果（表名/表值/表用途/表功能）：表名/表值/表用途/表功能（A 级实录）

## 4. 交付判据

考证文档（docs commit（D 面数据表考证第二十件（表名/表值/表用途/表功能（A 级实录））+**refs 引用**（bank:addr 引证（表读径实证（A 级原文））））——「候令」禁用。
