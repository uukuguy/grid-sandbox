---
name: code-debugger
description: 帮助调试代码问题，提供错误分析和修复建议
capabilities: [FileRead, ShellExec]
triggers: [debug, error, fix, bug, 调试, 错误]
---

# Code Debugger Skill

## 触发条件
用户请求调试代码、修复错误时触发。

## 执行步骤
1. 读取相关代码文件
2. 分析错误信息
3. 提供修复建议
4. 可选择执行修复命令

## 示例
- "帮我调试这个错误"
- "这个 bug 怎么修复"
- "代码出错了"
