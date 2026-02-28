---
name: code-review
description: 提供代码审查服务，分析代码质量并提出改进建议
capabilities: [FileRead, ShellExec]
triggers: [review, 代码审查, 审查, code review]
---

# Code Review Skill

## 触发条件
用户请求代码审查时触发。

## 执行步骤
1. 读取代码文件
2. 分析代码质量
3. 识别问题和建议
4. 提供改进方案

## 示例
- "帮我审查代码"
- "看看这个实现怎么样"
- "代码有什么问题"
