# frontend/ — React 19 + Vite 7

SPA 前端，从 Next.js 迁移而来。

## 技术栈

- React 19 + React Router v7
- Vite 7 + @vitejs/plugin-react
- TailwindCSS 4（@tailwindcss/vite 插件）
- TanStack Query v5（服务端状态）
- i18next（国际化）

## 目录结构

| 目录 | 职责 |
|------|------|
| `api/` | API client 层（client.ts 封装、auth.ts、sse.ts） |
| `components/ui/` | 通用 UI 组件（Glass UI 原语、图标、模态框） |
| `components/shared/` | 业务共享组件（资产创建/编辑模态框） |
| `contexts/` | React Context providers |
| `features/` | 按功能模块组织的页面逻辑 |
| `hooks/` | 自定义 Hooks |
| `i18n/` | 国际化配置 |
| `lib/` | 工具库（query-client, query-keys） |
| `routes/` | 页面组件（Landing, SignIn, SignUp, Workspace...） |
| `styles/` | 全局样式 |
| `types/` | TypeScript 类型定义 |

## 规范

- 组件文件：PascalCase（`MyComponent.tsx`）
- 工具文件：camelCase（`queryKeys.ts`）
- 类型检查：`npx tsc --noEmit`
- 构建：`npm run build`（tsc -b && vite build）
- 不引入 `lucide-react`，图标使用 `components/ui/icons/custom.tsx`（98 个自定义图标）
- 数据获取统一用 TanStack Query hooks，不直接在组件中 fetch

## Design Token v2（Glass UI 设计系统）

所有样式必须使用 CSS 变量，**禁止硬编码** Tailwind 圆角/颜色 class。

### 圆角（锐利风格）

| Token | 值 | 用途 |
|-------|-----|------|
| `--glass-radius-none` | 0px | 导航栏、工具栏 |
| `--glass-radius-xs` | 2px | 按钮、Chip、Input |
| `--glass-radius-sm` | 2px | 表单控件、小组件 |
| `--glass-radius-md` | 3px | 卡片（`.glass-surface`） |
| `--glass-radius-lg` | 4px | 面板、侧边栏 |
| `--glass-radius-xl` | 6px | 弹窗（Modal）— 最大值 |

**写法**：`rounded-[var(--glass-radius-md)]`，不要写 `rounded-lg` / `rounded-xl`。
**例外**：`rounded-full` 可用于圆形元素（头像、状态点、spinner、toggle）。

### 颜色

| Token | 值 | 用途 |
|-------|-----|------|
| `--glass-bg-canvas` | #f8f7f5 | 页面背景 |
| `--glass-text-primary` | #111111 | 主文本 |
| `--glass-text-secondary` | #6b6b6b | 次要文本 |
| `--glass-accent-from` | #e8553a | 强调色（CTA 按钮） |

### 密度（紧凑优先）

| 组件 | 高度 |
|------|------|
| 按钮 sm | h-7 (28px) |
| 按钮 md | h-8 (32px) |
| 按钮 lg | h-10 (40px) |
| Input default | h-9 (36px) |
| Input compact | h-8 (32px) |
| 导航栏 padding | py-2 |
| 卡片 padding | p-3 md:p-4 |

### 微动画

| 交互 | 实现 |
|------|------|
| 卡片 hover | `hover-lift` class（translateY -2px + shadow） |
| 卡片 press | `press-feedback` class（scale 0.98） |
| 按钮 hover | translateY(-1px)，180ms ease-out |
| 按钮 active | scale(0.98)，100ms（CSS 内置） |
| 图标按钮 hover | scale(1.08) |
| Modal 入场 | `animate-modal-in` class |
| 背景遮罩 | `animate-backdrop-in` class |
| Toast 滑入 | `animate-toast-in` class |
| 骨架屏 | `animate-skeleton` class |
| 生成中脉冲 | `animate-status-pulse` class |
| 列表交错入场 | `animate-list-item` + inline delay |
| 减弱动效 | 自动适配 `prefers-reduced-motion` |

### Transition Token

```css
--glass-ease-default: cubic-bezier(0.2, 0, 0, 1);
--glass-ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
--glass-ease-out: cubic-bezier(0.16, 1, 0.3, 1);
--glass-duration-fast: 150ms;
--glass-duration-normal: 200ms;
--glass-duration-slow: 300ms;
```

### ⚠️ 禁止项

- ❌ 不要使用 `rounded-xl` / `rounded-lg` / `rounded-2xl` 等硬编码 Tailwind 圆角
- ❌ 不要使用 inline `borderRadius` style（video-editor/Remotion 除外）
- ❌ 不要使用旧的弹跳 easing `cubic-bezier(0.68, -0.55, 0.265, 1.55)`
- ❌ 不要给 Chip 使用 `rounded-full`（应用 `--glass-radius-sm`）
- ✅ 应始终使用 `rounded-[var(--glass-radius-*)]` 格式
