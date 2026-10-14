# ROM 目录（BYO-ROM，仓库不含任何游戏数据）

Metroid (NES, 1986, Nintendo) 的 ROM 仍受版权保护，**本项目不下载、不收录、不分发 ROM**。
请使用你自行合法取得的卡带 dump（自购卡带 + dump 器，或你已有的合法备份），放入本目录。

## 期望版本与指纹（USA/NTSC，推荐基线）

| 项 | 值 |
|---|---|
| 文件名约定 | `Metroid (USA).nes` |
| 格式 | iNES，mapper 1 (MMC1)，PRG 128KB (8×16KB) + CHR 8KB |
| 整文件 CRC32 | `A2C89CB9` |
| headerless PRG+CHR CRC32 | `70080810` |
| 对应 m1disasm 构建目标 | `NES_NTSC` |

PAL/Europe 备选：整文件 `A591CC24`，headerless `7751588D`（目标 `NES_PAL`）。
指纹来源：[mstan/MetroidNESRecomp](https://github.com/mstan/MetroidNESRecomp) README。

## 校验

```bash
python3 tools/verify_rom.py "rom/Metroid (USA).nes"
```

脚本会解析 header、算哈希、比对指纹、按 bank 输出 CRC32。

## 隐私

ROM 文件已列入 `.gitignore`，永远不会被提交。任何验证产物（trace、快照）中也不得包含完整 ROM 数据。
