name: transformer-calibration
version: 1.0.0
description: |
  基于实时 SCADA 遥测数据为电力变压器进行阈值校准的可复用 skill。
  适用于任意变压器设备，通过读取历史数据并计算统计阈值来优化监控设置。
  支持 temperature、load、dissolved-gas 三种校准类型。

author: Grid Agent Team
license: MIT

metadata:
  category: industrial
  tags: [scada, calibration, transformer, threshold, monitoring]
  devices: [transformer, xfmr]
  supported_calibrations:
    - temperature
    - load  
    - dissolved-gas

hooks:
  pre_tool_use:
    - name: validate_device_id
      description: "验证设备 ID 格式是否正确"
    - name: check_calibration_type
      description: "验证校准类型是否受支持"
      
  post_tool_use:
    - name: record_calibration_snapshot
      description: "记录校准前的数据快照"
    - name: generate_calibration_report
      description: "生成校准报告并存储到内存"
      
  pre_edit:
    - name: require_calibration_context
      description: "确保有足够的校准上下文数据"

skills:
  - id: read_scada_snapshot
    name: 读取 SCADA 数据快照
    description: |
      从 SCADA 系统读取指定设备的历史遥测数据，用于校准分析。
      支持时间窗口配置，默认为 5 分钟回溯。
    parameters:
      device_id:
        type: string
        required: true
        pattern: "^xfmr-[0-9]{3,}$"
        description: "变压器设备 ID，格式为 xfmr-XXX"
      time_window:
        type: string
        required: false
        default: "5m"
        enum: ["5m", "15m", "1h", "24h"]
        description: "数据回溯时间窗口"
      calibration_type:
        type: string
        required: true
        enum: ["temperature", "load", "dissolved-gas"]
        description: "校准类型"
    outputs:
      snapshot_data:
        type: object
        description: "包含温度、负载或气体数据的数组"
      statistics:
        type: object
        description: "基本统计信息（最小值、最大值、平均值、标准差）"
    examples:
      - device_id: "xfmr-001"
        time_window: "15m"
        calibration_type: "temperature"

  - id: calculate_thresholds
    name: 计算校准阈值
    description: |
      基于读取的 SCADA 数据，计算统计阈值（如警告和报警阈值）。
      支持多种计算方法：percentile、std_dev、fixed_ratio。
    parameters:
      calibration_type:
        type: string
        required: true
        enum: ["temperature", "load", "dissolved-gas"]
      data_points:
        type: array
        required: true
        description: "从 SCADA 快照获取的数据点数组"
      method:
        type: string
        required: false
        default: "percentile"
        enum: ["percentile", "std_dev", "fixed_ratio"]
      warning_level:
        type: number
        required: false
        default: 90
        description: "警告阈值参数（百分位或标准差倍数）"
      critical_level:
        type: number
        required: false
        default: 95
        description: "临界阈值参数（百分位或标准差倍数）"
    outputs:
      warning_threshold:
        type: number
        description: "警告阈值"
      critical_threshold:
        type: number
        description: "临界阈值"
      confidence_score:
        type: number
        description: "置信度分数 (0-1)"
      recommended_action:
        type: string
        description: "基于阈值的推荐行动"
    examples:
      - calibration_type: "temperature"
        data_points: [65.2, 67.1, 66.8, 68.3, 69.1]
        method: "percentile"
        warning_level: 90
        critical_level: 95

  - id: validate_thresholds
    name: 验证阈值合理性
    description: |
      验证计算出的阈值是否在合理范围内，避免过低或过高的阈值。
      参考行业标准和设备规格书。
    parameters:
      calibration_type:
        type: string
        required: true
      warning_threshold:
        type: number
        required: true
      critical_threshold:
        type: number
        required: true
      device_specs:
        type: object
        required: false
        description: "设备规格参数（如额定容量、额定温度等）"
    outputs:
      is_valid:
        type: boolean
        description: "阈值是否有效"
      issues:
        type: array
        description: "验证发现的问题列表"
      adjustments:
        type: object
        description: "建议的调整值"
    examples:
      - calibration_type: "temperature"
        warning_threshold: 85.0
        critical_threshold: 95.0
        device_specs:
          rated_temperature: 110
          ambient_max: 40

  - id: generate_calibration_memory
    name: 生成校准记忆
    description: |
      将校准过程和结果存储为持久化记忆，供未来参考和审计。
    parameters:
      device_id:
        type: string
        required: true
      calibration_type:
        type: string
        required: true
      original_thresholds:
        type: object
        required: true
      new_thresholds:
        type: object
        required: true
      data_summary:
        type: object
        required: true
      timestamp:
        type: string
        required: true
      confidence_score:
        type: number
        required: true
    outputs:
      memory_id:
        type: string
        description: "存储的记忆 ID"
      archive_status:
        type: string
        description: "归档状态"
    examples:
      - device_id: "xfmr-001"
        calibration_type: "temperature"
        original_thresholds: { warning: 80, critical: 90 }
        new_thresholds: { warning: 85, critical: 95 }
        data_summary: { min: 65, max: 70, avg: 67.5, count: 100 }
        timestamp: "2026-04-18T00:00:00Z"
        confidence_score: 0.95

workflows:
  calibration_flow:
    description: "完整的变压器阈值校准工作流"
    steps:
      - name: "读取实时数据"
        skill: "read_scada_snapshot"
      - name: "计算统计阈值"
        skill: "calculate_thresholds"
      - name: "验证阈值合理性"
        skill: "validate_thresholds"
      - name: "存储校准结果"
        skill: "generate_calibration_memory"
    error_handling:
      - "数据不足 → 延长时间窗口重试"
      - "验证失败 → 使用保守默认值并记录"
      - "存储失败 → 重试最多 3 次"

safety:
  read_only: true
  no_write_operations: true
  evidence_required: true
  audit_log: true

notes: |
  此 skill 仅用于阈值计算和校准建议，不执行实际的 SCADA 写操作。
  所有阈值变更需要人工审批和独立的工作流执行。
  校准结果应定期验证，至少每季度重新计算一次阈值。
