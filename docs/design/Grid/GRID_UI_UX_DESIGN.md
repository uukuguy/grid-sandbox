# Grid UI/UX 设计规范

> 文档版本: 1.0 | 日期: 2026-04-03
> 状态: 草案 (Draft)
> 适用范围: Grid Studio TUI 模式 + Grid Studio Web 模式

---

## 一、设计原则

### 1.1 五条核心原则

| # | 原则 | 说明 | 反面模式 |
|---|------|------|---------|
| 1 | **深度层次** | 通过背景色梯度和边框创造"浮出"视觉感 | 所有元素在同一平面 |
| 2 | **品牌贯穿** | Indigo 品牌色从 Welcome 到状态栏一以贯之 | Welcome 金色 → 对话蓝色的色调突变 |
| 3 | **信息密度** | 开发者工具偏好高密度，14px 正文，紧凑间距 | 消费级 App 的大留白 |
| 4 | **动效克制** | 只在状态变化时动画，150-300ms，不装饰 | 循环闪烁、弹跳图标 |
| 5 | **一致性** | TUI 和 Web 共享相同的语义色、图标含义、信息层级 | TUI 绿色表成功，Web 蓝色表成功 |

### 1.2 跨平台一致性矩阵

| 元素 | TUI 实现 | Web 实现 | 一致性要求 |
|------|---------|---------|-----------|
| 品牌色 | `theme.accent` RGB | CSS `--color-primary` | 必须相同色值 |
| 语义色 | `theme.success/error/warning` | CSS `--color-success/error/warning` | 必须相同色值 |
| 品牌图标 | `◆` (U+25C6) | `<svg>◆</svg>` | 相同符号 |
| 状态栏信息 | 模型/Token/Git/Context% | 相同信息 | 布局可不同，信息必须同 |
| 工具结果 | 折叠/展开 (Ctrl+O) | 折叠/展开 (点击) | 交互方式不同，行为一致 |
| 消息角色标识 | `> ` 用户 / `⏺ ` 助手 | 头像+气泡 | 视觉不同，语义一致 |

---

## 二、品牌视觉系统

### 2.1 品牌色

```
主品牌色 (Brand):
  #5E6AD2  RGB(94, 106, 210)     Indigo — 成熟、科技、专业
  用途: Welcome 动效、活跃指示、品牌标识

品牌暗色 (Brand Dim):
  #2F3569  RGB(47, 53, 105)
  用途: 不活跃边框、背景纹理

品牌发光 (Brand Glow):
  rgba(94, 106, 210, 0.15)
  用途: hover 发光效果、聚焦指示器
```

### 2.2 品牌图标

| 场景 | TUI | Web | 说明 |
|------|-----|-----|------|
| 状态栏品牌 | `◆ Grid` | `<GridIcon /> Grid` | ◆ = U+25C6 实心菱形 |
| Welcome 标题 | 圆角线条 ASCII Art | SVG Logo | 见 §3.6 |
| Favicon | — | ◆ 矢量 SVG 16x16 | 与 TUI 品牌符号一致 |

**不使用 emoji**：🦑 等 emoji 在终端中宽度不稳定，不受 fg 颜色控制，违反 no-emoji-icons 规范。

### 2.3 ASCII Art Logo（TUI 专用）

**Tier 3（h ≥ 12 行）— 圆角线条风格**：

```
  ╭───╮  ╭───╮  ╶──╮  ╭───╮
  │      │   │     │  │    │
  │ ──╮  ├───╯     │  │    │
  │   │  │  ╲      │  │    │
  ╰───╯  ╵   ╲  ╶──╯  ╰───╯
```

- 宽度: ~34 字符
- 颜色: `brand` 色 HSL 呼吸渐变（色相 230-260°）
- 网格背景: `brand_dim` 色径向脉冲点阵

**Tier 2（h 5-11 行）— 极简三行**：

```
╔═╗  ╦═╗  ╦  ╔══╗
║ ╗  ╠╦╝  ║  ║  ║
╚═╝  ╩╚═  ╩  ╚══╝
```

**Tier 1（h < 5 行）— 纯文字**：

```
◆ grid — autonomous ai workbench
```

---

## 三、色彩系统

### 3.1 统一色彩 Token 定义

TUI (`theme.rs`) 和 Web (`globals.css`) **必须使用相同的 RGB 值**。

#### 背景层级（4 层深度）

| Token | RGB | 用途 | TUI 字段 | Web CSS |
|-------|-----|------|---------|---------|
| `bg-deep` | `#0a0a0f` RGB(10,10,15) | 最深底层 | `surface` | `--color-background` |
| `surface-1` | `#111118` RGB(17,17,24) | 卡片/面板 | `surface_highlight` | `--color-card` |
| `surface-2` | `#1a1a24` RGB(26,26,36) | 悬浮层/弹窗 | (新增) | `--color-surface-2` |
| `surface-3` | `#242430` RGB(36,36,48) | 输入框/下拉 | (新增) | `--color-input` |

**核心原则**：每层亮度差 5-8%，创造"浮出"视觉感。

#### 文字层级（3 层）

| Token | RGB | 用途 | TUI 字段 | Web CSS |
|-------|-----|------|---------|---------|
| `fg` | `#EDEDEF` RGB(237,237,239) | 主文字 | `text` | `--color-foreground` |
| `fg-muted` | `#8A8F98` RGB(138,143,152) | 次要文字 | `text_secondary` | `--color-muted-foreground` |
| `fg-faint` | `#4E5158` RGB(78,81,88) | 占位符/禁用 | (新增) | `--color-faint` |

#### 边框

| Token | 值 | 用途 |
|-------|-----|------|
| `border` | `rgba(255, 255, 255, 0.08)` | 默认边框 |
| `border-hover` | `rgba(255, 255, 255, 0.15)` | hover 状态 |

#### 语义色（跨平台一致）

| Token | RGB | 用途 |
|-------|-----|------|
| `success` | `#22C55E` RGB(34,197,94) | 成功、已完成、运行中 |
| `warning` | `#F59E0B` RGB(245,158,11) | 警告、注意 |
| `error` | `#EF4444` RGB(239,68,68) | 错误、失败、危险操作 |
| `info` | `#3B82F6` RGB(59,130,246) | 信息提示 |

### 3.2 当前问题及修复

**问题: 两套色彩系统并存**

| `theme.rs` 语义色 | `style_tokens.rs` 常量 | 差异 |
|-------------------|----------------------|------|
| `success: RGB(34,197,94)` | `SUCCESS: RGB(106,209,143)` | 不一致 |
| `error: RGB(239,68,68)` | `ERROR: RGB(255,92,87)` | 不一致 |
| `warning: RGB(234,179,8)` | `WARNING: RGB(255,179,71)` | 不一致 |
| `border: RGB(51,65,85)` | `BORDER: RGB(88,88,88)` | 不一致 |

**修复方案**：`style_tokens.rs` 的语义色改为与 `theme.rs` 一致，或改为从 TuiTheme 实例获取。

### 3.3 Web CSS 修复

当前 `globals.css` 的问题：`secondary/muted/accent/input/border` 6 个 token 复用同一个 `#27272a`。

**修复后的 `globals.css`**：

```css
@theme {
  --color-background:         #0a0a0f;
  --color-foreground:         #EDEDEF;
  --color-card:               #111118;
  --color-card-foreground:    #EDEDEF;
  --color-primary:            #5E6AD2;
  --color-primary-foreground: #FFFFFF;
  --color-secondary:          #1a1a24;
  --color-secondary-foreground: #EDEDEF;
  --color-muted:              #1a1a24;
  --color-muted-foreground:   #8A8F98;
  --color-accent:             #5E6AD2;
  --color-accent-foreground:  #FFFFFF;
  --color-destructive:        #EF4444;
  --color-border:             rgba(255, 255, 255, 0.08);
  --color-input:              #242430;
  --color-ring:               #5E6AD2;
  --radius:                   0.5rem;
}
```

### 3.4 TUI 主题 Accent 色调整

12 个可选主题保持不变，但**默认主题从 Cyan 改为 Indigo**：

```rust
// theme.rs
impl Default for ThemeName {
    fn default() -> Self { ThemeName::Indigo }  // 改自 Cyan
}
```

Welcome 面板的动效色相随之调整：
- 当前: HSL hue 25-55°（Amber 暖金区）
- 建议: HSL hue 230-260°（Indigo 蓝紫区），与默认 accent 统一

---

## 四、排版系统

### 4.1 Web 字体

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');

body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
}

.font-mono, code, pre {
  font-family: 'JetBrains Mono', ui-monospace, monospace;
}
```

### 4.2 Type Scale

| Token | 大小 | Weight | 用途 |
|-------|------|--------|------|
| `text-xs` | 12px | 400 | 标签、徽章、时间戳 |
| `text-sm` | 13px | 400 | 辅助信息、工具参数 |
| `text-base` | 14px | 400 | **正文**（开发者工具偏小） |
| `text-lg` | 16px | 500 | 小标题 |
| `text-xl` | 20px | 600 | 页面标题 |
| `text-2xl` | 24px | 700 | 大标题 |

**行高**: 正文 1.5，标题 1.3

### 4.3 TUI 排版

TUI 无法控制字体，但控制字符宽度和间距：

| 元素 | 规则 |
|------|------|
| 消息缩进 | 2 空格 `"  "` |
| 工具结果缩进 | 5 空格 `"     "` |
| 思考块续行 | `"│ "` 竖线 + 空格 |
| 嵌套深度 | 每级 2 空格，最大 4 级 |

---

## 五、图标系统

### 5.1 Web 图标库

使用 **Lucide React** (`lucide-react`)，统一 stroke-width=1.5。

| 用途 | 图标 | 组件 |
|------|------|------|
| 聊天 | `MessageSquare` | 导航 |
| 任务 | `ListTodo` | 导航 |
| 工具 | `Wrench` | 导航 |
| 记忆 | `Brain` | 导航 |
| 调试 | `Bug` | 导航 |
| MCP | `Server` | 导航 |
| 设置 | `Settings` | 导航 |
| 发送 | `Send` | 输入框 |
| 加载 | `Loader2` | 动画旋转 |
| 成功 | `Check` | 状态 |
| 错误 | `X` | 状态 |
| 警告 | `AlertTriangle` | 状态 |

**禁止**: 任何 emoji 作为结构性图标（导航、状态、控件）。

### 5.2 TUI 图标符号

| 图标 | Unicode | 用途 | 着色 |
|------|---------|------|------|
| `◆` | U+25C6 | 品牌标识 | `brand` |
| `⏺` | U+23FA | 助手消息前缀 | `accent` |
| `>` | ASCII | 用户消息前缀 | `info` |
| `▸` | U+25B8 | 工具调用标记 | `warning` |
| `⚙` | U+2699 | 折叠工具名 | `fg-muted` |
| `✓` | U+2713 | 成功 | `success` |
| `✗` | U+2717 | 失败 | `error` |
| `⟡` | U+27E1 | 思考中 | `magenta` |
| `⏇` | U+23C7 | Git 分支 | `success`/`warning` |
| `⏱` | U+23F1 | 耗时 | `fg-muted` |
| `▮` | U+25AE | 进度条-已填充 | 按百分比变色 |
| `▯` | U+25AF | 进度条-未填充 | `border` |
| `·` | U+00B7 | 分隔符 | `border` |

### 5.3 MCP 服务器状态（TUI + Web 统一）

| 状态 | 当前(Web) | 修改后(统一) |
|------|----------|-------------|
| 运行中 | 🟢 emoji | `●` (U+25CF) + `success` 色 |
| 已停止 | ⚪ emoji | `○` (U+25CB) + `fg-muted` 色 |
| 错误 | 🔴 emoji | `●` (U+25CF) + `error` 色 |
| 启动中 | ⏳ emoji | `◌` (U+25CC) + `warning` 色 |

---

## 六、布局系统

### 6.1 TUI 布局（垂直栈）

```
┌──────────────────────────────────────────┐
│  CONVERSATION AREA                       │  Min 5 行, flex 填充
│  (滚动区域: 消息 + 工具结果)              │
├──────────────────────────────────────────┤
│  ACTIVITY INDICATOR                      │  0 或 1 行 (Streaming/Thinking)
├──────────────────────────────────────────┤
│  ── separator ──────────────────────── ──│  1 行 (模式标签 + 帮助提示)
│  ❯ input text                            │  1-8 行 (多行输入)
├──────────────────────────────────────────┤
│  ──────────────────────────────────────  │  分隔线
│  ◆ Grid │ model │ ▸in ▾out │ ━━━── 80% │  信息行 1
│  …/path │ ⏇ branch +1 ~2               │  信息行 2
└──────────────────────────────────────────┘
```

**动态高度约束**：
```
input_lines = buffer.lines().max(1).min(8) + 1  // +1 分隔符
activity = if streaming || thinking { 1 } else { 0 }
status = 4  // 固定: 分隔线 + 2 信息行 + 空行
conversation = 剩余空间, 最小 5 行
```

### 6.2 Web 布局（侧边栏 + 内容区）

```
┌──────┬──────────────────────────────────┐
│ ◆    │                                  │
│      │                                  │
│ 💬   │     主内容区                      │
│ Chat │     (各页面组件)                  │
│      │                                  │
│ 📋   │                                  │
│ Tasks│                                  │
│      │                                  │
│ 🔧   │                                  │
│ Tools│                                  │
│      │                                  │
│ 🧠   │                                  │
│Memory│                                  │
│      │                                  │
│ 🐛   │                                  │
│ Debug│                                  │
│      │                                  │
│ 🔌   │                                  │
│ MCP  │                                  │
│      │                                  │
│ ──── │                                  │
│ ⚙    │                                  │
└──────┴──────────────────────────────────┘
```

**侧边栏规格**：
- 折叠模式: 48px 宽（仅图标）
- 展开模式: 200px 宽（图标 + 标签）
- 活跃项: 左侧 2px `primary` 色指示条 + 背景高亮
- 底部: Settings 固定

**间距系统**：4px 基础单位，8px 倍数递进

```
gap-1 = 4px    组件内间距
gap-2 = 8px    元素间距
gap-3 = 12px   区块内间距
gap-4 = 16px   区块间距
gap-6 = 24px   节段间距
gap-8 = 32px   页面级间距
```

---

## 七、组件规范

### 7.1 卡片

**Web**:
```css
.card {
  background: var(--color-card);         /* surface-1 */
  border: 1px solid var(--color-border); /* rgba(255,255,255,0.08) */
  border-radius: 8px;
  transition: all 150ms ease;
}

.card:hover {
  border-color: var(--color-border-hover);
  box-shadow: 0 0 0 1px var(--color-border-hover),
              0 4px 16px rgba(0, 0, 0, 0.4);
}
```

**TUI**:
```rust
// 普通边框
theme.styled_block("Title")       // border: theme.border
// 活跃/聚焦边框
theme.styled_block_active("Title") // border: theme.accent
```

### 7.2 按钮

**Web**:

| 变体 | 样式 |
|------|------|
| Primary | `bg-primary text-white rounded-md px-4 py-2 hover:opacity-90 active:scale-[0.97]` |
| Secondary | `bg-secondary border border-border hover:bg-secondary/80` |
| Destructive | `bg-error text-white hover:opacity-90` |
| Ghost | `hover:bg-surface-2` |

**通用**: `transition: all 150ms ease; cursor: pointer;`

### 7.3 输入框

**Web**:
```css
.input {
  background: var(--color-input);        /* surface-3 */
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px 12px;
  font-size: 14px;
  color: var(--color-foreground);
  transition: border-color 150ms ease;
}

.input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--brand-glow);
}
```

**TUI**:
```
❯ 输入文本█       ← accent 色光标
  续行文本         ← 2 空格缩进
```

- 光标: `bg(theme.accent)` 而非固定白色
- 禁用态: `fg-faint` 灰色 + 不响应输入

### 7.4 状态徽章

**统一语义（TUI + Web）**:

| 状态 | 颜色 | TUI | Web |
|------|------|-----|-----|
| Running | `success` | `● running` | `<Badge variant="success">Running</Badge>` |
| Completed | `info` | `✓ completed` | `<Badge variant="info">Completed</Badge>` |
| Failed | `error` | `✗ failed` | `<Badge variant="error">Failed</Badge>` |
| Pending | `warning` | `◌ pending` | `<Badge variant="warning">Pending</Badge>` |
| Blocked | `fg-muted` | `◌ blocked` | `<Badge variant="muted">Blocked</Badge>` |

### 7.5 工具结果区块

**TUI 折叠态**:
```
▸ ⚙ bash ✓ 1.2s — 15 lines (Ctrl+O cycle)
```

**TUI 展开态**:
```
╭─ bash ──────────────────── 1.2s ─╮
│ $ ls -la                          │
│ total 128                         │
│ drwxr-xr-x  12 user ...          │
╰──────────────────────────────────╯
```

- 顶部边框: `accent_dim` 色
- 工具名: `bold`
- 耗时: `fg-muted` 右对齐
- 内容: 最大 20 行，超出显示 `(N more lines)`
- 成功图标 `✓` 用 `success` 色，失败 `✗` 用 `error` 色

**Web**: 相同折叠/展开行为，点击切换。

### 7.6 进度条

**Context 使用量（状态栏）**:

```
TUI:  ━━━━━━── 75%     (8 段, accent 色填充, border 色空)
Web:  ████████░░ 75%    (10 段, CSS 渐变)
```

**颜色阈值（统一）**:
- `> 50%` 剩余: `success` 绿色
- `25-50%` 剩余: `warning` 金色
- `< 25%` 剩余: `error` 橙红色

### 7.7 Toast 通知（Web）

```
╭─ ✓ Session saved ──────────╮
│  Saved to ~/.grid/sessions  │
╰────────────────────────────╯
```

- 位置: 右上角
- 自动消失: 3-5 秒
- 成功: 左边框 `success` 色
- 错误: 左边框 `error` 色
- `aria-live="polite"` 无障碍

### 7.8 骨架屏（Web）

```tsx
function Skeleton({ className }: { className?: string }) {
  return (
    <div className={cn(
      "animate-pulse rounded-md bg-[var(--color-secondary)]",
      className
    )} />
  );
}
```

用于: Chat 加载、Tools 列表加载、Memory 搜索结果加载

---

## 八、动效规范

### 8.1 通用原则

| 规则 | 值 | 说明 |
|------|-----|------|
| 微交互时长 | 150ms | hover、active、toggle |
| 组件过渡时长 | 200-300ms | 页面切换、展开/折叠 |
| 最大时长 | 400ms | 复杂动画上限 |
| 缓动函数 | `cubic-bezier(0.16, 1, 0.3, 1)` | Expo.out，自然减速 |
| 退出动画 | 入场时长 × 0.7 | 退出要比进入快 |
| `prefers-reduced-motion` | 必须尊重 | 关闭所有非必要动画 |

### 8.2 TUI 动效

#### 呼吸动效（Welcome + Activity Indicator）

```rust
// HSL 正弦波驱动亮度变化
let phase = (elapsed_ms as f64 / 2000.0) * TAU;  // 2 秒周期
let lightness = 0.55 + 0.15 * phase.sin();       // 40%-70% 亮度区间
let color = hsl_to_rgb(hue, saturation, lightness);
```

- Welcome 面板: 边框 + 标题 + 网格同频呼吸
- Activity Indicator: spinner 字符 + elapsed 时间同频呼吸

#### 渐变扫光（Welcome 标题）

```rust
let sweep = (char_idx * 4 + line_offset * 12 + 360 - gradient_offset) % 360;
let hue = 230.0 + (sweep as f64 / 360.0) * 30.0;  // 蓝紫色相区间
// gradient_offset 每 tick +3，360 次循环一圈
```

#### Spinner 动画

Braille 点阵 10 帧循环，100ms/帧：
```
⠋ → ⠙ → ⠹ → ⠸ → ⠼ → ⠴ → ⠦ → ⠧ → ⠇ → ⠏
```

### 8.3 Web 动效

#### 页面切换

```css
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}
.page-enter { animation: fadeIn 200ms var(--easing); }
```

#### 卡片 hover

```css
.card {
  transition: all 150ms var(--easing);
}
.card:hover {
  transform: translateY(-1px);
  border-color: var(--color-border-hover);
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}
```

#### 按钮按下

```css
.btn:active {
  transform: scale(0.97);
}
```

#### 骨架屏

```css
.skeleton {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50%      { opacity: 0.5; }
}
```

---

## 九、信息层级规范

### 9.1 状态栏信息优先级

**TUI 状态栏**（2 行，从左到右信息递减）：

```
行 1: ◆ Grid │ claude-sonnet-4 │ ▸1.5k ▾2.3k │ ⏱ 2m5s │ ● development │ ━━━━── 60%
      品牌     模型名(accent色)  Token 消耗     耗时      沙箱环境         Context 剩余

行 2: …/project-path │ ⏇ main +1 ~2 ?3 ↑0
      工作目录          Git 分支及状态
```

**Web 状态区**：相同信息，布局可不同（水平 bar 或面板）。

### 9.2 消息渲染层级

| 角色 | TUI 样式 | Web 样式 |
|------|---------|---------|
| **用户** | `> ` 蓝色粗体前缀 | 右对齐气泡，`surface-2` 背景 |
| **助手** | `⏺ ` accent 色前缀 | 左对齐，无背景 |
| **系统** | `! ` 灰色斜体 | 居中灰色小字 |
| **思考** | `⟡ ` 洋红色，`│` 续行 | 可折叠区域，斜体灰色 |
| **工具调用** | `▸ verb(arg)` 金色 | 卡片式，带图标 |
| **工具结果** | 折叠/展开框 | 折叠/展开框 |

**消息间距**：角色切换时插入 1 行空行。

---

## 十、无障碍规范

### 10.1 对比度

| 元素 | 前景 | 背景 | 对比度 | 标准 |
|------|------|------|--------|------|
| 主文字 | `#EDEDEF` | `#0a0a0f` | 16.8:1 | ✅ AAA |
| 次要文字 | `#8A8F98` | `#0a0a0f` | 6.2:1 | ✅ AA |
| 品牌色文字 | `#5E6AD2` | `#0a0a0f` | 4.7:1 | ✅ AA |
| 占位符 | `#4E5158` | `#0a0a0f` | 3.2:1 | ✅ 大文本 AA |

### 10.2 TUI 无障碍

| 功能 | 状态 | 说明 |
|------|------|------|
| IME 输入 | ✅ 已支持 | Display width 计算 CJK 宽度 |
| Keyboard navigation | ✅ | Ctrl+O 折叠/展开、Ctrl+C 中断 |
| 终端最小尺寸 | ✅ | Tier 1/2/3 自适应 |
| 高对比度 | ⚠️ 需改进 | 光标应用 accent 色而非固定白 |

### 10.3 Web 无障碍

| 功能 | 要求 |
|------|------|
| Focus ring | 2px `brand-glow` 色 box-shadow |
| Tab 顺序 | 匹配视觉顺序 |
| ARIA labels | 图标按钮必须有 `aria-label` |
| 表单 label | 每个输入框有可见 label |
| Toast | `aria-live="polite"` |
| 错误消息 | `role="alert"` |
| 颜色不唯一 | 状态同时用颜色 + 图标 + 文字 |

---

## 十一、改进清单（按优先级）

### P0 — 一致性修复（影响品牌统一）

| # | 改动 | TUI 文件 | Web 文件 | 改动量 |
|---|------|---------|---------|--------|
| 1 | 统一语义色值 (success/error/warning/border) | `style_tokens.rs` | `globals.css` | ~10 行 |
| 2 | Autocomplete 弹窗用主题色 | `render.rs:207-238` | — | ~8 行 |
| 3 | 光标颜色跟主题 | `input.rs` | — | ~2 行 |
| 4 | 模型名用 accent 色 | `status_bar.rs:261` | — | ~1 行 |
| 5 | Web 背景色 4 层化 | — | `globals.css` | ~15 行 |
| 6 | MCP 状态 emoji → Unicode 符号 | — | MCP 组件 | ~4 处 |

### P1 — 精致度提升

| # | 改动 | 适用 | 预期效果 |
|---|------|------|---------|
| 7 | Context 进度条 5→8 段 + 精细字符 | TUI | 数据展示精度 |
| 8 | 工具结果框加耗时 + accent 边框 | TUI | 信息更丰富 |
| 9 | 消息间加空行分隔 | TUI+Web | 视觉层次 |
| 10 | Welcome ASCII Art 改圆角线条 GRID | TUI | 品牌升级 |
| 11 | 品牌图标 🦑→◆ | TUI | 终端兼容性 |
| 12 | Web 引入 Inter + JetBrains Mono | Web | 专业感 |
| 13 | Web 导航 TabBar → 侧边栏 | Web | 现代感 |

### P2 — 体验增强

| # | 改动 | 适用 | 预期效果 |
|---|------|------|---------|
| 14 | 骨架屏组件 | Web | 感知性能 |
| 15 | 页面切换 fadeIn | Web | 流畅感 |
| 16 | 卡片 hover 浮起 + 发光 | Web | 交互反馈 |
| 17 | Toast 通知系统 | Web | 操作反馈 |
| 18 | 默认主题 Cyan→Indigo | TUI | 品牌统一 |
| 19 | Welcome 动效色相 Amber→Indigo | TUI | 品牌统一 |
| 20 | 运行时主题切换 (Ctrl+T) | TUI | 高级体验 |

---

## 十二、验收检查清单

### TUI 验收

- [ ] 12 个主题均能正常渲染，无硬编码颜色泄漏
- [ ] Welcome 面板 3 个 Tier 在不同终端高度下正确切换
- [ ] ASCII Art "GRID" 使用圆角线条，品牌色呼吸
- [ ] 状态栏品牌图标为 ◆，不使用 emoji
- [ ] Autocomplete 弹窗颜色跟随主题
- [ ] 光标颜色跟随主题
- [ ] 工具结果折叠/展开正常，包含耗时信息
- [ ] Context 进度条按阈值变色（绿→金→红）
- [ ] 消息间有角色切换空行

### Web 验收

- [ ] 背景色呈现 4 层深度（查看 DevTools 确认不同 RGB）
- [ ] 品牌色 #5E6AD2 贯穿（导航高亮、按钮、链接）
- [ ] 无 emoji 作为结构图标（导航、状态、控件）
- [ ] Inter 字体正确加载（检查 Network → Fonts）
- [ ] 所有卡片 hover 有视觉反馈（translateY + 边框变亮）
- [ ] 输入框 focus 有品牌色发光环
- [ ] 对比度 ≥ 4.5:1（主文字对背景）
- [ ] 骨架屏在加载时正确显示

### 跨平台一致性验收

- [ ] TUI `theme.success` 和 Web `--color-success` 为相同 RGB
- [ ] TUI 和 Web 的工具调用状态图标含义一致
- [ ] TUI 和 Web 的 Context 进度条颜色阈值一致
