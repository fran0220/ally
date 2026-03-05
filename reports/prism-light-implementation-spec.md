# Prism Light Refined — Implementation Spec

## Design Token Mapping (旧 → 新)

### 核心色板
| 用途 | 旧值 | 新值 |
|------|------|------|
| Canvas | #f3f4f6 | #faf9f7 |
| Surface | rgba(255,255,255,0.88) | #ffffff (实色) |
| Surface Strong | rgba(255,255,255,0.94) | #ffffff |
| Surface Modal | rgba(255,255,255,0.97) | #ffffff |
| Muted | rgba(255,255,255,0.86) | #f5f4f2 |
| Nav | rgba(255,255,255,0.96) | #ffffff |
| Text Primary | #0a0a0a | #1a1a1a |
| Text Secondary | #111827 | #737373 |
| Text Tertiary | #4b5563 | #a0a0a0 |
| Text On Accent | #ffffff | #ffffff |

### 边框
| 用途 | 旧值 | 新值 |
|------|------|------|
| Stroke Soft | rgba(255,255,255,0.22) | #f0ede8 |
| Stroke Base | rgba(111,126,153,0.24) | #e8e4df |
| Stroke Strong | rgba(93,109,138,0.36) | #d4d0ca |
| Stroke Focus | rgba(47,123,255,0.64) | #e8553a |
| Stroke Danger | rgba(236,72,72,0.64) | #dc3545 |
| Stroke Warning | rgba(234,149,0,0.62) | #e6930e |
| Stroke Success | rgba(18,176,109,0.62) | #0fa968 |

### 阴影 (暖色调)
| 用途 | 新值 |
|------|------|
| Shadow SM | 0 1px 3px rgba(160,140,120,0.06) |
| Shadow MD | 0 4px 12px rgba(150,130,110,0.08) |
| Shadow LG | 0 8px 24px rgba(140,120,100,0.10) |
| Shadow Modal | 0 16px 48px rgba(140,120,100,0.14) |
| Shadow Nav | 0 1px 0 #e8e4df |
| Blur 全部 | 0px（移除所有 backdrop-filter） |

### 圆角
| 用途 | 旧值 | 新值 |
|------|------|------|
| XS | 8px | 8px |
| SM | 12px | 10px |
| MD | 16px | 12px |
| LG | 22px | 16px |
| XL | 28px | 20px |

### Accent (暖珊瑚色，克制使用)
| 用途 | 旧值 | 新值 |
|------|------|------|
| Accent From | #2f7bff | #e8553a |
| Accent To | #5ca8ff | #e8553a (不用渐变) |
| Accent Shadow Soft | rgba(47,123,255,0.24) | rgba(232,85,58,0.12) |
| Accent Shadow Strong | rgba(47,123,255,0.32) | rgba(232,85,58,0.20) |
| Focus Ring | rgba(47,123,255,0.16) | rgba(232,85,58,0.10) |
| Focus Ring Strong | rgba(47,123,255,0.22) | rgba(232,85,58,0.18) |
| Ghost Hover BG | rgba(255,255,255,0.5) | #f5f4f2 |

### Tone System
| Tone | BG | FG |
|------|----|----|
| Neutral | #f5f4f2 | #737373 |
| Info | #eef4ff | #3b7dd8 |
| Success | #eefbf4 | #0d8a55 |
| Warning | #fef8ec | #b87400 |
| Danger | #fef2f2 | #c53030 |

### Overlay
| 用途 | 新值 |
|------|------|
| Soft | rgba(26,26,26,0.20) |
| Default | rgba(26,26,26,0.40) |
| Strong | rgba(26,26,26,0.60) |

---

## 语义 CSS 变更要点

1. **移除所有 `backdrop-filter` 和 `-webkit-backdrop-filter`**
2. **Surface 类** — 使用实色背景 + 暖阴影
3. **按钮 Primary** — `background: #e8553a`（不用渐变），hover 用 `#d44a32`
4. **按钮 Secondary** — `background: #ffffff`, border: `1px solid #e8e4df`
5. **按钮 Soft** — `background: #f5f4f2`
6. **按钮 Ghost** — 透明, hover → `#f5f4f2`
7. **按钮 Danger** — `background: #fef2f2`, `color: #c53030`
8. **Input Focus** — `box-shadow: 0 0 0 2px rgba(232,85,58,0.18)`
9. **Nav** — 底部 1px 分割线代替 shadow（`box-shadow: 0 1px 0 #e8e4df`）
10. **Chip** — pill shape (`border-radius: 999px`)，实色背景
11. **Interactive hover** — `translateY(-1px)` with `cubic-bezier(0.68, -0.55, 0.265, 1.55)` 弹性

---

## 动画变更

1. 保留核心 keyframes: fadeIn, fadeInDown, slideUp, scaleIn, slideInRight, pageSlideIn/Out, shimmer, progressSweep
2. 移除不需要的: aurora, blob（Glass morphism 装饰效果）
3. 更新 duration/easing:
   - fadeIn: 200ms ease-out
   - fadeInDown: 350ms cubic-bezier(0.68, -0.55, 0.265, 1.55)
   - slideUp: 250ms cubic-bezier(0.68, -0.55, 0.265, 1.55)
   - scaleIn: 200ms ease-out
   - pageSlideIn: 300ms cubic-bezier(0.16, 1, 0.3, 1)

---

## globals.css 变更

1. Body 背景：移除 radial-gradient 装饰 blob，改为纯色 `#faf9f7`
2. 字体：`"Noto Sans SC", "PingFang SC", system-ui, sans-serif`，`font-weight: 400`
3. `.page-shell` — 添加响应式 padding: `width: min(1440px, calc(100% - 48px))`，移动端 `calc(100% - 32px)`
4. `.glass-nav` — 移除 backdrop-filter, 改用底部细线
5. `.glass-page-title` — 调整字重 `600`
6. `.glass-kpi` — 实色白底 + 暖阴影

---

## 响应式设计变更

### Thread 1: Navbar 移动端汉堡菜单
- 文件: `frontend/src/components/Navbar.tsx`
- 移动端 (< md): 汉堡图标按钮，点击展开全屏/侧边导航
- 桌面端 (≥ md): 保持现有水平布局
- 添加 `useState` 控制 `mobileMenuOpen`
- 移动端菜单: 从顶部滑入, 全宽导航链接, 44px 触摸目标

### Thread 2: 页面响应式
- **WorkspaceList**: 项目卡片 grid `grid-cols-1 sm:grid-cols-2 xl:grid-cols-3`（已基本有，确认）
- **ProjectWorkbench**: 9 个 stage tabs → 移动端水平滚动 `overflow-x-auto`, 侧栏 episodes 移动端折叠
- **AssetHub**: 侧栏 folders 移动端折叠 + KPI 卡片 `grid-cols-1 sm:grid-cols-3`
- **Landing**: feature cards `grid-cols-1 md:grid-cols-3`（已有），CTA 按钮移动端全宽

### Thread 3: GlassSurface interactive hover
- 更新 `GlassSurface.tsx` 的 interactive hover transition 到新弹性曲线
- 确保所有原语组件中的 `var(--glass-*)` 引用仍然有效

---

## 实施线程分配

| Thread | 范围 | 文件 |
|--------|------|------|
| A: CSS Token + Semantic + Animations + Globals | 4 个样式文件 | ui-tokens-glass.css, ui-semantic-glass.css, animations.css, globals.css |
| B: Navbar 响应式 | 汉堡菜单 | Navbar.tsx |
| C: 页面响应式 + 原语微调 | 4 个页面 + 1 个原语 | ProjectWorkbench.tsx, AssetHub.tsx, Landing.tsx, WorkspaceList.tsx, GlassSurface.tsx |

## 验证命令
```bash
cd frontend && npx tsc --noEmit && npm run build
```
