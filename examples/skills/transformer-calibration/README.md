# Transformer Calibration Skill

## 概述

这是一个基于实时 SCADA 遥测数据为电力变压器进行阈值校准的可复用 skill。适用于任意变压器设备，通过读取历史数据并计算统计阈值来优化监控设置。

## 功能特性

- 支持三种校准类型：`temperature`（温度）、`load`（负载）、`dissolved-gas`（溶解气体）
- 灵活的时间窗口配置（5分钟、15分钟、1小时、24小时）
- 多种阈值计算方法（百分位数、标准差、固定比例）
- 自动验证阈值合理性
- 完整的记忆存储和审计追踪

## 使用方法

### 1. 读取 SCADA 数据快照

```bash
# 读取过去 15 分钟的温度数据
skill: read_scada_snapshot
parameters:
  device_id: "xfmr-001"
  time_window: "15m"
  calibration_type: "temperature"
```

### 2. 计算统计阈值

```bash
# 使用百分位方法计算阈值
skill: calculate_thresholds
parameters:
  calibration_type: "temperature"
  data_points: [65.2, 67.1, 66.8, 68.3, 69.1, ...]
  method: "percentile"
  warning_level: 90
  critical_level: 95
```

### 3. 验证阈值合理性

```bash
# 验证计算出的阈值是否合理
skill: validate_thresholds
parameters:
  calibration_type: "temperature"
  warning_threshold: 85.0
  critical_threshold: 95.0
  device_specs:
    rated_temperature: 110
    ambient_max: 40
```

### 4. 生成校准记忆

```bash
# 将校准结果存储为持久化记忆
skill: generate_calibration_memory
parameters:
  device_id: "xfmr-001"
  calibration_type: "temperature"
  original_thresholds: { warning: 80, critical: 90 }
  new_thresholds: { warning: 85, critical: 95 }
  data_summary: { min: 65, max: 70, avg: 67.5, count: 100 }
  timestamp: "2026-04-18T00:00:00Z"
  confidence_score: 0.95
```

## 工作流

完整的校准工作流包含以下步骤：

1. **读取实时数据** - 从 SCADA 系统获取历史遥测数据
2. **计算统计阈值** - 基于数据计算警告和临界阈值
3. **验证阈值合理性** - 确保阈值在合理范围内
4. **存储校准结果** - 将结果存储为持久化记忆

## 错误处理

- **数据不足** → 延长时间窗口重试
- **验证失败** → 使用保守默认值并记录
- **存储失败** → 重试最多 3 次

## 安全特性

- **只读操作**：此 skill 仅用于阈值计算，不执行 SCADA 写操作
- **证据要求**：所有校准决策都需要数据支持
- **审计追踪**：完整记录所有校准活动

## Hooks 说明

### Pre-Tool-Use Hooks
- `validate_device_id` - 验证设备 ID 格式
- `check_calibration_type` - 验证校准类型

### Post-Tool-Use Hooks
- `record_calibration_snapshot` - 记录校准前的数据快照
- `generate_calibration_report` - 生成校准报告

### Pre-Edit Hooks
- `require_calibration_context` - 确保有足够的校准上下文数据

## 设备 ID 格式

变压器设备 ID 必须遵循以下格式：
- 格式：`xfmr-XXX`
- 示例：`xfmr-001`、`xfmr-042`、`xfmr-123`

## 注意事项

1. 此 skill 仅用于阈值计算和校准建议，不执行实际的 SCADA 写操作
2. 所有阈值变更需要人工审批和独立的工作流执行
3. 校准结果应定期验证，至少每季度重新计算一次阈值
4. 确保设备 ID 格式正确，否则将被拒绝

## 版本

- 当前版本：1.0.0
- 作者：Grid Agent Team
- 许可证：MIT
