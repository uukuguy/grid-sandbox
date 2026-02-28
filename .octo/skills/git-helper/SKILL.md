---
name: git-helper
description: 帮助处理 Git 操作，提供提交、分支、合并等操作指导
capabilities: [ShellExec, FileRead]
triggers: [git, commit, branch, merge, pr, Git]
---

# Git Helper Skill

## 触发条件
用户请求 Git 相关操作时触发。

## 执行步骤
1. 理解用户请求的 Git 操作
2. 执行相应的 git 命令
3. 解释操作结果
4. 提供后续建议

## 示例
- "帮我提交代码"
- "创建一个分支"
- "如何解决合并冲突"
- "查看 git 历史"
