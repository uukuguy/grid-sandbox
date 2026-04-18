# Transformer Calibration Skill 使用示例

## 完整校准流程示例

### 场景：为变压器 xfmr-001 进行温度阈值校准

#### 步骤 1：读取 SCADA 数据快照

```json
{
  "skill": "read_scada_snapshot",
  "parameters": {
    "device_id": "xfmr-001",
    "time_window": "15m",
    "calibration_type": "temperature"
  }
}
```

**预期输出：**
```json
{
  "snapshot_data": {
    "temperature": [65.2, 67.1, 66.8, 68.3, 69.1, 68.8, 67.5, 66.9, 68.2, 69.0]
  },
  "statistics": {
    "min": 65.2,
    "max": 69.1,
    "avg": 67.69,
    "std_dev": 1.23,
    "count": 10
  }
}
```

#### 步骤 2：计算统计阈值

```json
{
  "skill": "calculate_thresholds",
  "parameters": {
    "calibration_type": "temperature",
    "data_points": [65.2, 67.1, 66.8, 68.3, 69.1, 68.8, 67.5, 66.9, 68.2, 69.0],
    "method": "percentile",
    "warning_level": 90,
    "critical_level": 95
  }
}
```

**预期输出：**
```json
{
  "warning_threshold": 68.9,
  "critical_threshold": 69.1,
  "confidence_score": 0.92,
  "recommended_action": "Thresholds within acceptable range. Consider quarterly review."
}
```

#### 步骤 3：验证阈值合理性

```json
{
  "skill": "validate_thresholds",
  "parameters": {
    "calibration_type": "temperature",
    "warning_threshold": 68.9,
    "critical_threshold": 69.1,
    "device_specs": {
      "rated_temperature": 110,
      "ambient_max": 40
    }
  }
}
```

**预期输出：**
```json
{
  "is_valid": true,
  "issues": [],
  "adjustments": {}
}
```

#### 步骤 4：生成校准记忆

```json
{
  "skill": "generate_calibration_memory",
  "parameters": {
    "device_id": "xfmr-001",
    "calibration_type": "temperature",
    "original_thresholds": {
      "warning": 80,
      "critical": 90
    },
    "new_thresholds": {
      "warning": 68.9,
      "critical": 69.1
    },
    "data_summary": {
      "min": 65.2,
      "max": 69.1,
      "avg": 67.69,
      "count": 10
    },
    "timestamp": "2026-04-18T00:00:00Z",
    "confidence_score": 0.92
  }
}
```

**预期输出：**
```json
{
  "memory_id": "calib-xfmr-001-temp-20260418-000000",
  "archive_status": "confirmed"
}
```

## 不同校准类型示例

### 负载校准

```json
{
  "skill": "read_scada_snapshot",
  "parameters": {
    "device_id": "xfmr-042",
    "time_window": "1h",
    "calibration_type": "load"
  }
}
```

### 溶解气体校准

```json
{
  "skill": "read_scada_snapshot",
  "parameters": {
    "device_id": "xfmr-123",
    "time_window": "24h",
    "calibration_type": "dissolved-gas"
  }
}
```

## 不同计算方法示例

### 标准差方法

```json
{
  "skill": "calculate_thresholds",
  "parameters": {
    "calibration_type": "temperature",
    "data_points": [...],
    "method": "std_dev",
    "warning_level": 2.0,
    "critical_level": 3.0
  }
}
```

### 固定比例方法

```json
{
  "skill": "calculate_thresholds",
  "parameters": {
    "calibration_type": "load",
    "data_points": [...],
    "method": "fixed_ratio",
    "warning_level": 0.85,
    "critical_level": 0.95
  }
}
```

## 常见错误处理

### 错误 1：无效的设备 ID

```bash
ERROR: Invalid device_id format. Expected: xfmr-XXX (e.g., xfmr-001)
ERROR: Got: transformer-001
```

**解决方案：** 使用正确的设备 ID 格式 `xfmr-001`

### 错误 2：不支持的校准类型

```bash
ERROR: Unsupported calibration_type: voltage
ERROR: Supported types: temperature, load, dissolved-gas
```

**解决方案：** 仅使用支持的校准类型之一

### 警告：数据点不足

```bash
WARNING: Limited data points (8). Consider increasing time window.
```

**解决方案：** 增加时间窗口（如从 `5m` 改为 `15m` 或 `1h`）

## 工作流配置

在 `SKILL.md` 中定义的 `calibration_flow` 工作流可以自动执行所有步骤：

```yaml
workflows:
  calibration_flow:
    steps:
      - name: "读取实时数据"
        skill: "read_scada_snapshot"
      - name: "计算统计阈值"
        skill: "calculate_thresholds"
      - name: "验证阈值合理性"
        skill: "validate_thresholds"
      - name: "存储校准结果"
        skill: "generate_calibration_memory"
```

## 注意事项

1. **数据质量：** 确保从 SCADA 系统读取的数据质量良好，无异常值
2. **时间窗口选择：** 根据设备运行特性和监控需求选择合适的时间窗口
3. **验证结果：** 始终验证计算出的阈值是否符合设备规格和行业标准
4. **定期更新：** 建议至少每季度重新计算一次阈值
5. **人工审批：** 所有阈值变更都需要人工审批后才能应用
