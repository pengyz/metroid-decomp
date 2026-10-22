# 事故②法医取证（2026-01-17 11:57 采集）

## 僵尸进程（核心物证）
```
2545800 2545796 S       12:34:52 python -m ida_pro_mcp.idalib_supervisor --host 127.0.0.1 --port 13337 --unsafe
2962238 2545800 Zs      06:17:08 [python] <defunct>
systemd(1)---systemd(4664)---bash(2545796)---python(2545800)---python(2962238)
```

## 假说（待对抗验证）
ida_pro_mcp.idalib_supervisor（port 13337，IDA/REA 分析桥）的 worker 05:43 死后未收割
→ 监管器半残态 → 后续 MCP 分析调用挂起 → 子 agent turn 挂死 → 工作流卡死（两次同窗）

## 待查面
- 谁/何时调用过 idapro/REA 分析工具（teammate 的 MCP 调用日志）
- 我们脚本层超时覆盖矩阵（task-38 后的漏网面）
- gates 总闸执行路径时长/挂死面
- 阻塞原语全仓清单
