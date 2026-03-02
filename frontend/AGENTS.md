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
