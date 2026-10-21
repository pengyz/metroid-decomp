# 案 id 对齐考证（manifest 映射扩（case id 映射表））

## 1. 案 id 对齐考证（a3-028-case-N vs l2_cases.jsonl 案 id 匹配）

### 1.1 manifest 映射扩（case id 映射表（a3-028-case-N→l2_cases.jsonl id））

**案 id 命名规**（l2-c16d-set-ptr-lo-hi（族-序-名形））：
- 族：l2（L2 子程序级）
- 序：c16d（bank:addr 序）
- 名：set-ptr-lo-hi（功能名）

### 1.2 案 id 形实录（3107 案（id 字段形））

| 案 id | 形式 | 备注 |
|---|---|---|
| l2-c16d-set-ptr-lo-hi | 族-序-名形 | 3107 案 id 形 |
| l2-c376-absx-zero-store | 族-序-名形 | 3107 案 id 形 |
| l2-c376-absx-x0 | 族-序-名形 | 3107 案 id 形 |

## 2. 案 id 对齐文档（可落码形（anchor_provenance.py inject（manifest 映射（case_id→capture_id/trace_digest））））

**case id 映射表**（可落码形（anchor_provenance.py inject（manifest 映射（case_id→capture_id/trace_digest））））：
- case_id→capture_id：manifest 映射（case id→capture id）
- case_id→trace_digest：manifest 映射（case id→trace digest）

## 3. 交付判据

案 id 对齐实录（A 级实录（case id 映射表（案 id 形实录）））+docs commit——「候令」禁用。
