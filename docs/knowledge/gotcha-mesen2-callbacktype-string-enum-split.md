---
type: gotcha
---

# gotcha：Mesen2 字符串 "execute" 与 callbackType.exec 枚举语义分叉（r2 F1 怪象全解）

## 现象
`emu.addMemoryCallback(fn, "execute", lo, hi)`（字符串注册）与
`emu.addMemoryCallback(fn, emu.callbackType.exec, lo, hi)`（枚举注册）**语义不同**：

| 注册 | 语义 | C0BF（每帧确证执行的 JSR opcode） | C0D9（NMI 入口） | pos2 操作数字节 |
|---|---|---|---|---|
| 字符串 "execute" | 劣化：操作数/数据读 | **0 命中** | **0 命中** | 可见 |
| `callbackType.exec` | **真 opcode fetch** | **55/60 帧** | **59/60 帧** | 0 |

r2-coverage-mechanism-review F1 的全部怪象由此解释（混操作数字节、漏 NMI/reset 入口
首指令）——**结论「指令级覆盖不可得」应更正为「字符串注册不可得，枚举注册可得」**。
配套：内存回调内 `getState()["cpu.pc"]`=回调地址回显（非真 PC）——pc 过滤无效；
站点签名校验（栈 ret-1 反推 site + 校验 `20 7C C2` 字节）是回调内自证的可靠手段
（raw=valid=10968/10968）。

## 教训
- **枚举表要用枚举名**：`emu.callbackType`/`eventType`/`stepType` 是正牌 API 面；
  字符串参数是兼容层/别名，语义可能劣化——**字符串 vs 枚举双注册对照**是 API 语义
  取证的标准动作（v7 探针 5 行对照换一个根治口）；
- 与 gotcha-operand-type-by-opcode-byte 同族：**语义以实现为准，不以名为准**；
  探针数据先过不变量证伪（环⑤）再信——本案探针 v1 的 885 事件全是假命中；
- `emu.*` 全表面（pairs 枚举）值得先挖：getCdlData/getAccessCounters/step/stepType
  在 r2 时点已存在，被「API 不可得」结论挡住没挖。

## 复现（2026-10-15，probe_dispatch_diag v4/v7）
```sh
xvfb-run -a tools/mesen2/Mesen "rom/Metroid (USA).nes" tools/probe_dispatch_diag.lua /DoNotSaveSettings
cat analysis/probe_dispatch_diag7.log
# count C0BF: str=0 enum=55 | count C0D9: str=0 enum=59 | count C27C: str=165 enum=55
```

## 高危同族
- 任何「字符串参数 vs 枚举参数」双形 API（callbackType/read memType 类）；
- 用回调内 getState 做过滤器前先验真值性（pc 回显案）；
- 「某能力不可得」类否定结论先挖 API 全表再定论。
