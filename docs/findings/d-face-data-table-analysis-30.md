# D 面数据表考证第三十件（L4 就绪批微工单——Samus 域数据表（跳跃/变球））

## 1. D 面数据表考证第三十件（ObjAnim_SamusExplode（01:85a4（common_sprite_data.asm（40））））

### 1.1 refs asm 考证（bank:addr（表读径（表值（表内容（表用途（表功能考证）））））

**bank:addr**（表读径（表值（表内容（表用途（表功能考证（refs asm（数据表使用径（bank:addr（表读径（grep 实录（数据表使用（子程序（表读指令））））））））））：
- ObjAnim_SamusExplode（01:85a4）：动画数据（Samus 爆炸动画（ObjAnim（动画表索引）））
- prg0_title.asm（439）：ObjAnim_SamusExplode（动画数据（Samus 爆炸动画（ObjAnim（动画表索引））））

### 1.2 refs 引用必写（bank:addr 引证（表读径实证（A 级原文）））

**refs 引用必写**（bank:addr 引证（表读径实证（A 级原文）））：
- bank:addr 引证（表读径实证（A 级原文（refs asm（bank:addr（表读指令实证）））））
- common_sprite_data.asm（40）：ObjAnim_SamusExplode（动画数据（Samus 爆炸动画（ObjAnim（动画表索引））））

## 2. 考证文档（表名/表值/表用途/表功能（A 级实录））

| 表名 | 表值 | 表用途 | 表功能 |
|---|---|---|---|
| ObjAnim_SamusExplode | 01:85a4 | 动画数据（Samus 爆炸动画（ObjAnim（动画表索引））） | 表功能考证（refs asm（数据表使用径）） |

## 3. 更新留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能））））

**追溯径**（refs 引用（bank:addr）+考证成果（表名/表值/表用途/表功能））：
- refs 引用（bank:addr）：bank:addr 引证（表读径实证（A 级原文））
- 考证成果（表名/表值/表用途/表功能）：表名/表值/表用途/表功能（A 级实录）

## 4. 交付判据

考证文档（docs commit（D 面数据表考证第三十件（表名/表值/表用途/表功能（A 级实录））+**refs 引用**（bank:addr 引证（表读径实证（A 级原文））））——「候令」禁用。
