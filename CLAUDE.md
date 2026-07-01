# JM Boom 代码审计与改进建议

本文档记录一次全项目扫描的结果，涵盖前端模块拆分、Rust 端性能、样式规范和依赖库使用四个维度。所有条目附带文件路径和行号，供后续改造时定位。

## 一、代码不优雅 / 模块拆分不合理

### 1.1 路由文件承担业务实现

以下路由把页面实现内嵌在路由文件里，与 `settings.tsx`、`downloads.tsx`、`comic/$comicId.tsx`（5-13 行薄壳 + `features/*/page.tsx`）风格不统一：

- `src/routes/_app/index.tsx`（380 行）
- `src/routes/_app/list.tsx`（378 行）
- `src/routes/_app/weekly.tsx`
- `src/routes/_app/search.tsx`
- `src/routes/_app/ranking.tsx`
- `src/routes/_app/favorites.tsx`
- `src/routes/_app/history.tsx`
- `src/routes/_app/me.tsx`

建议：全部收敛成薄壳，实现挪到 `src/features/home/`、`features/list/`、`features/history/` 等。

### 1.2 单文件过大 / 职责过多

- `src/features/settings/page.tsx`（328 行）：一个组件挂了 14 个 query/mutation。拆成 `use-settings-endpoints.ts`、`use-settings-cache.ts`、`use-settings-account.ts`、`use-settings-updates.ts`、`use-settings-diagnostics.ts`。
- `src/features/reader/reader-strip-window.tsx`（264 行）：`ReaderStripImage` 是第二个 god-component 且带独立 `useQuery`，拆到 `reader-strip-image.tsx`。
- `src/features/reader/reader-page.tsx`：history upsert effect（L65-95）和 `goBack`/`scrollStripBy` 应拆到 `use-reader-history-sync.ts`、`use-reader-navigation-actions.ts`。
- `src/features/comic-detail/page.tsx`：详情缓存回写、收藏 mutation、评论 `useInfiniteQuery`、下载 drawer 全塞在一起。拆 `use-comic-detail-actions.ts`、`use-comic-comments.ts`。

### 1.3 Store 膨胀

- `src/stores/settings-store.ts`（226 行）：reader 相关字段占了一半（`readerReadMode`、`readerPageDirection`、`readerDoublePageMode`、`readerAutoRead*` 等）。`partialize` 手动同步很脆。拆成 `reader-settings-store.ts` 或 slice。

### 1.4 重复实现

- `currentChinaWeekday` 在 `_app/index.tsx:123` 和 `_app/list.tsx:331` 复制了两份。挪到 `src/lib/route-search.ts`。
- `OverflowTooltipTitle` 在 `components/comic-feed.tsx:89` 和 `downloads/download-task-card.tsx:170` 逐字复制。
- `BackTop` / `useBackTopVisibility` 在 `_app/index.tsx:199-326` 和 `comic-detail/shared.tsx:10-133` 各写了一份，只差 threshold。
- 布局 padding `p-[32px_32px_16px_96px]` 出现在 10 处，抽 `AppPageLayout`。
- 列表页 `useQuery({ staleTime, gcTime, refetchOnMount: false, refetchOnWindowFocus: false })` 每个路由复制一遍，抽 `useListQuery` helper。
- `replace: true, resetScroll: false, search: { ...search, page: 1, X: value }` 分页更新在 `list/weekly/ranking/search` 四份，抽 `useSearchUpdater`。

### 1.5 冗余 / 死代码

- `features/reader/reader-strip-window.tsx:180-188` 用 `useEffect(shouldPreload → setIsNearViewport(true))` 让状态实际只写不读，直接 `shouldLoad = isNearViewport || shouldPreload` 即可。
- `features/comic-detail/download-drawer.tsx:19` `DownloadChapterOption = DownloadChapterRequest` 是空别名，直接用 API 类型。
- `features/comic-detail/shared.tsx:8` 重导出 `ComicCover` 无意义。
- `routes/_app/me.tsx:31-51` 已经有 `use-me-sign-in.ts`，但 auto-sign-in effect 还留在路由里。

## 二、Rust 端性能没释放

### 2.1 最高性价比的三个点

1. **下载持久化 O(N) 全表重写**：`src-tauri/src/download/storage.rs:135-195` `persist_tasks_to_pool` 每秒都 `DELETE FROM download_tasks` + 全量 INSERT。多任务时是 O(tasks × chapters) 全表重写。改成只 upsert 变化那一条，IOPS 骤降。
2. **下载并发过保守 + 阶段串行**：`src-tauri/src/download/manager.rs:28` `DOWNLOAD_PAGE_CONCURRENCY = 4` 偏保守，CDN 通常能扛 8-16。`manager.rs:380-401` `for chapter in &task.chapters` 串行 `get_or_load_manifest`，应用 `JoinSet` 并发拉。`manager.rs:449-500` 建议先并发下 bytes，再用 rayon 并行 decode/encode。
3. **热路径冗余拷贝**：`src-tauri/src/reader/page.rs:286` `bytes.to_vec()` 白拷一次，可直接把 `Bytes` 传进 `spawn_blocking`（Arc 计数）；`image_decode.rs:20` `to_rgb8()` 没匹配 `ImageRgb8(_)` 直接借用，Rgb8 源被无谓复制；`WebPMemory` 可直接 `fs::write(&*mem)`，省一份 Vec。

### 2.2 其他明显问题

- `Cargo.toml:33` `tokio` 只启了 `rt`（单线程），并行 `spawn_blocking` 会被限流；确认是否想要 `rt-multi-thread`。
- `Cargo.toml:31` reqwest 没启 `gzip`/`brotli`/`deflate`，API JSON 白付带宽。
- `src-tauri/src/reader/image_decode.rs:26-56` `reorder_scrambled_rgb_rows` 外层每段目的地互不重叠，天然可 `rayon::par_iter` 并行 memcpy。
- `src-tauri/src/api/setting.rs:41-62` endpoint 探测串行 `for endpoint in candidates`，应并发 + `select` 最快者。
- `src-tauri/src/api/client.rs:33-77` `SHARED_HTTP_CLIENT` 用 `Mutex<Option<Client>>`，每次请求都锁；改 `OnceLock<Client>` 或 `ArcSwap` 免锁。
- `src-tauri/src/api.rs:53` `JWT_TOKEN: Mutex<Option<String>>` 每次拼 header 都 lock+clone，`ArcSwap<Option<Arc<String>>>` 免锁读。
- `src-tauri/src/reader/manifest.rs:224-235` `cached_manifest` 是全局 `Mutex<HashMap>` + clone 整个 manifest，DashMap 或 RwLock 明显更好。
- `src-tauri/src/reader/page.rs:24` `PAGE_MATERIALIZE_LOCKS` HashMap 只写不清理，长会话内存泄漏。
- `src-tauri/src/storage/db.rs:26` `max_connections: 5`，下载 4 并发 + 读线程 + cleanup 会互相排队，建议 8-16。
- `src-tauri/src/storage/runtime_cache.rs` 从未清理过期项，长期只增；set 时 opportunistic `DELETE WHERE expires_at < ?`。
- `src-tauri/src/reader/cache_index.rs:56-106` 每次写入都开事务 DELETE+INSERT，`ON CONFLICT` 已能 upsert，冗余；L187-209 trim 用 `SELECT *` 全表取，可 `LIMIT` 迭代。

## 三、样式不规范

### 3.1 Reader 硬编码调色板（最高影响）

Reader 大量硬写 `bg-neutral-950`、`text-neutral-50`、`text-neutral-400`、`white/10`、`black/80`，浅色模式下 reader 也是死黑。应定义 `.reader` scope + 局部 CSS 变量，或使用 theme token。集中在：

- `features/reader/reader-page.tsx:157`
- `reader-bars.tsx:31, 39, 47, 48, 55, 96`
- `reader-chapter-drawer.tsx:54, 56, 57, 65, 87, 88, 93`
- `reader-settings-menu.tsx`（十余处）
- `reader-progress-slider.tsx:38, 50, 70`
- `reader-strip-window.tsx:122, 223, 239, 241, 242, 248, 258`
- `reader-image.tsx:149, 160`
- `reader-state.tsx:5, 16`

### 3.2 Reader 外的硬色

- `routes/_app/history.tsx:128` `bg-black/40` 无 dark 变体，深色下消失。
- `components/theme-toggle.tsx:25, 44, 55` inline `stroke="#888"`、`fill="#424242"/#fff"`，改 `currentColor` 就跟随按钮 variant。

### 3.3 className 拼接

- `features/comic-detail/hero.tsx:258` 用模板字符串拼 className，应改成 `cn('flex …', className)`。

### 3.4 Z-index 混乱

`z-10/20/30/40/50` 到处飘且没有分层规范。FAB（`_app/index.tsx:212`、`comic-detail/shared.tsx:23`）和 shadcn Dialog 都用 `z-50`，会打架。建议在 theme 里定 `z-nav`、`z-fab`、`z-overlay`、`z-toast`。

### 3.5 抽屉宽度重复

`w-[420px]/440px/460px/480px` + `max-w-[calc(100vw-24px|48px)]` 在 `comments.tsx:37`、`download-drawer.tsx:77`、`reader-chapter-drawer.tsx:54`、`reader-bars.tsx:96` 里各自写一份，且和 Tailwind 内置 `w-96`/`w-[28rem]` 混用，抽 `SideDrawer` 变体。

### 3.6 shadcn 组件自造

- `features/reader/reader-progress-slider.tsx` 手写 `<input type="range">` + `::-webkit-slider-thumb`/`::-moz-range-thumb`，shadcn Slider（Radix）就是干这个的但没装。
- `reader-hot-zones.tsx:20, 29` 裸 `<button>` 加 `bg-transparent p-0 border-0`，用 shadcn `Button variant="ghost"` 即可。

### 3.7 状态色内联

`features/settings/api-endpoint-section.tsx:170, 174, 177` 直接写 `text-emerald-600 dark:text-emerald-400`。抽 `--color-success` / `--color-warning` 语义 token，否则会到处扩散。

## 四、库使用不好 / 自造轮子

### 4.1 应该装但没装的

- **`@tanstack/react-virtual`**：`features/reader/reader-strip-window.tsx:29-140` 是一个约 260 行的手写垂直虚拟列表 + `IntersectionObserver`。这是 react-virtual 的标准场景。
- **`usehooks-ts`**：`OverflowTooltipTitle` 里的 ResizeObserver+rAF 复制两份（`comic-feed.tsx:89-115`、`download-task-card.tsx:170-217`）；strip-window 里的 IntersectionObserver 也是。
- **`react-hotkeys-hook`**：`features/reader/use-reader-keyboard-navigation.ts:24-71` 是裸 `window.addEventListener('keydown')`，改声明式 `useHotkeys('ArrowLeft', ...)` 后 helper（L84-90）也能删。
- **`react-hook-form` + `zod`**：settings 页所有 input 都是散装 `useState` + 自定义 validator（`settings-store.ts:220-225` 的 `clampNumber`、`api-endpoint-section.tsx` 的 URL 校验、`account-section.tsx` 的账号密码校验），表单一多必然重复。

### 4.2 手写但应该抽掉

- `clampNumber` / `Math.min(Math.max(...))` 在 `settings-store.ts:220`、`comic-detail/chapters.tsx:44`、`reader-progress-slider.tsx:21`、`use-reader-navigation.ts:20-24, 60`（一个文件五处）。抽 `src/lib/math.ts`。
- Debounce/timeout 手写：`_app.tsx:92-94`、`account-section.tsx:53-61`、`providers.tsx:58-79`、`reader-chapter-drawer.tsx:40-49`。`use-reader-auto-read.ts:120-128` 的 setInterval-in-effect 也是。装 usehooks-ts 里的 `useDebounceCallback` / `useInterval`。
- `withTimeout` 在 `lib/api/search.ts:119-136` 手写，`AbortSignal.timeout()` 一行搞定。
- `formatBytes`/`formatDuration` 在 `lib/format.ts:1-27` 手写，且"秒/分钟/小时"字符串在 `download-task-card.tsx` 又拼了一遍。

### 4.3 TanStack Query 用得不到位

- `queryClient.setQueryData` 手动写在 `features/settings/page.tsx` 里 6 处（L75/98/108/136/166/169）、`comic-detail/page.tsx` 2 处、`use-download-tasks.ts:81`、`login-dialog.tsx:77`，多数应改成 `useMutation` 的 `onSuccess` + `invalidateQueries`。
- `providers.tsx:52-80` 用 `useEffect` 手动拉更新检查然后 `setQueryData`，直接 `useQuery({ queryKey: ['app-update'], enabled: delayReached })` 就行。
- `use-reader-auto-read.ts` 用 6 个 ref + 3 个 useEffect 追 "was enabled/paused/controls-visible"，其实是 `usePrevious` 或 zustand transient 的活。

### 4.4 Rust 侧手动 walk

- `plugin_codec.rs:12-71` 和 `reader/manifest.rs:46, 386` 用 `serde_json::Value` 手动 walk（`get("data").as_str()` 之类），应用 `#[serde(untagged)]` enum 变体十行搞定。
- `reader/cache_index.rs:48-51, 203-206`、`download/storage.rs:89-115`、`storage/session.rs:37-40` 手 `row.get("path"), i64_to_u64(row.get("size_bytes"))`，改 `#[derive(FromRow)]` + `sqlx::query_as`。
- 重试策略散在 `api/error.rs:37` 的 `is_retryable` 周围各自实现，没装 `reqwest-retry`/`backoff`。
