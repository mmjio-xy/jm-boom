# JM Boom Project Notes

本文件给后续接手的 Codex 使用，目标是快速理解项目约束、当前架构。内容应保持短而准，避免记录已经完成的过程流水账。

## 接手约定

- 项目是 Tauri 2 + React 19 + Vite + TanStack Router/Query + Zustand + shadcn/ui + Rust reqwest。
- 当前优先桌面端体验，小屏适配不是近期重点。
- 不要手动编辑 `src/routeTree.gen.ts`，路由树由 TanStack Router 插件、HMR 或 build 生成。
- 读取或写入包含中文的文件时必须显式使用 UTF-8，避免 Windows 终端默认编码导致乱码。
- 可以并行读取文件或搜索；文件编辑使用单次 `apply_patch`，不要重复或并行套同一补丁。
- `cargo check`、`vite build`、`tsc` 等会占用输出目录或 build lock 的验证命令不要并行重复触发。
- 前端和 Rust 通信用 Tauri `invoke`，项目不使用 axios。
- UI 优先复用 shadcn/ui 已安装组件；需要新的 shadcn 组件时，告诉用户安装，不要自己重造复杂基础组件。
- 项目已有 `floating-nav`，不要新增传统 sidebar。
- Rust 依赖可直接添加并用 `cargo check` 下载；前端依赖不要直接安装，先告诉用户。
- 用户此前表示暂时不要纠结 `lint`；不要因为 `lint` 结果扩大无关改动。
- 每次大的变动后，同步用户相应的 `commit msg`。
- 项目处于高速迭代期，不需要历史兼容包袱；重构可直接替换旧实现，不保留旧 JSON/旧 schema/旧 fallback 的兼容分支。

## 验证命令

- 常规前端验证优先跑 `bun run build`。
- 如果 Windows 环境没有 `bun`，跑：
  - `node_modules/.bin/tsc.exe --noEmit`
  - `node_modules/.bin/vite.exe build`
- 改 `src-tauri` Rust 代码后跑：
  - 工作目录 `src-tauri`
  - `cargo check`
- 只有在新增或调整测试时再跑相关 `cargo test`。

## 发布流程

- 发布前必须同步修改三处版本号：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`。
- 改完 Rust crate 版本后运行 `cargo check`，更新并验证 `src-tauri/Cargo.lock` 中的 `jm-boom` 版本。
- 每个发布 tag 必须有对应发布日志：`docs/release-notes/<tag>.md`，例如 `docs/release-notes/v0.2.2.md`。
- 发布日志按上一个 tag 到本次发布的 commit 生成，建议先看提交范围：
  - `git describe --tags --abbrev=0`
  - `git log <previous-tag>..HEAD --oneline`
- GitHub Actions 会校验发布日志包含以下四个板块，为空则不保留标题：
  - `## 功能`
  - `## 优化`
  - `## 修复`
  - `## 其他`
- 发布日志不需要带类似 “# JM Boom v0.2.2” 的版本号。
- 每条发布日志记录尽量带对应 commit hash 链接，格式示例：`- 设置页进入时自动检查更新 ([abc1234](https://github.com/ppxb/jm-boom/commit/abc1234))`。
- TAURI_SIGNING_PRIVATE_KEY 用户已完成配置，发布由用户手动通过 Github Workflow 完成。

## 参考项目

- API / 数据源主要参考同级 `Breeze-plugin-JmComic`。
- `Breeze` 更适合作为桌面/跨平台阅读器排版与翻页参考。

## 项目结构

- 前端路由在 `src/routes`。
- 普通页面在 `_app` 下，阅读页是独立路由 `src/routes/reader/$comicId.tsx`，不挂 `_app`，所以没有 `floating-nav`。
- 全局用户状态在 `src/stores/user-store.ts`。
- 设置状态在 `src/stores/settings-store.ts`，使用 Zustand persist，当前包含 API endpoint、阅读缓存上限、代理和封面隐私模式。
- 阅读历史在 `src/stores/reading-history-store.ts`，persist key 是 `jm-boom-reading-history`。
- 设置页路由 `src/routes/_app/settings.tsx` 仅保留入口，页面实现拆在 `src/features/settings/`。
- 下载页路由 `src/routes/_app/downloads.tsx` 仅保留入口，页面实现拆在 `src/features/downloads/`。
- Rust 后端模块已拆分：
  - `src-tauri/src/api.rs` + `src-tauri/src/api/*`：远程设置、搜索、首页、列表、排行、收藏、详情、评论、登录、签到、代理等普通接口。
  - `src-tauri/src/reader.rs` + `src-tauri/src/reader/*`：阅读 manifest、图片下载、解扰、缓存、reader cache 管理。
  - `src-tauri/src/download.rs` + `src-tauri/src/download/*`：下载任务状态机、持久化、路径规则和进度估算。
  - `src-tauri/src/storage/*`：SQLite 连接池、schema 初始化和后续本地持久化表。
  - `src-tauri/src/updater.rs`：应用更新检查和安装。
  - `src-tauri/src/lib.rs`：Tauri command 包装和 invoke handler 注册。

## 当前功能

- 首页：`src/routes/_app/index.tsx`，使用 `getHomeFeed`，信息流纵向展示，右侧轻量导航，含 BackTop。
- 列表页：`src/routes/_app/list.tsx`，复用 `getHomeSectionList`，承接首页分区、周榜、排行等列表入口。
- 每周推荐：`src/routes/_app/weekly.tsx`，使用 shadcn `Tabs`/`Select`，items 用 `ComicGrid`。
- 排行榜：`src/routes/_app/ranking.tsx`，使用 `getHomeSectionList` + `ranking-filters`。
- 收藏：`src/routes/_app/favorites.tsx`，依赖登录态，使用 `getFavoriteComics`。
- 详情页：`src/routes/_app/comic/$comicId.tsx`，展示详情、章节、相关推荐、评论 Drawer，支持收藏切换。
- 历史观看：`src/routes/_app/history.tsx`，以作品 `comicId` 为唯一键覆盖历史，`chapterId` 保存当前章节/read id；点击历史进入 `/reader/$comicId` 并带 `pageIndex` 恢复进度。
- 阅读页：`src/routes/reader/$comicId.tsx` + `src/features/reader/*`，`$comicId` 语义是 read/chapter id，作品 id 来自 search `albumId`。
- 设置页：`src/routes/_app/settings.tsx` + `src/features/settings/*`，动态发现 API endpoint，支持 HTTP/SOCKS5 代理、主题、封面隐私、缓存上限、缓存管理、诊断日志和应用更新。
- 个人中心：`src/routes/_app/me.tsx`，登录态来自 Zustand，支持签到数据和签到操作。
- 下载页：`src/routes/_app/downloads.tsx` + `src/features/downloads/*`，展示下载任务、筛选状态、暂停/恢复/取消/删除任务和打开目录。

## Rust / Tauri API 要点

- 主要 commands：`get_remote_setting`、`discover_api_endpoints`、`search_comics`、`get_home_feed`、`get_home_section_list`、`get_week_filters`、`get_week_items`、`get_comic_detail`、`toggle_comic_favorite`、`get_favorite_comics`、`get_comic_comments`、`login`、`get_sign_in_data`、`sign_in`、`clear_session`、`configure_network_proxy`、`get_reader_cache_stats`、`clear_reader_cache`、`open_reader_cache_dir`、`get_comic_read_manifest`、`get_comic_read_page`、`enqueue_comic_download`、`list_download_tasks`、`cancel_download_task`、`pause_download_task`、`resume_download_task`、`remove_download_task`、`open_download_task_dir`、`open_download_root_dir`、`check_app_update`、`install_app_update`。
- 移动端 API `tokenparam` 版本对齐 Breeze 插件为 `2.0.20`。
- API endpoint 动态发现从 `newsvr-2025.txt` 解密 host 池，合并本地 fallback 后测速；业务请求只规范化 http/https endpoint，空 endpoint 回退到 `https://www.cdnhjk.net`。
- Rust 侧支持全局代理配置，`configure_network_proxy` 会重建共享 reqwest client；HTTP/SOCKS5 依赖 reqwest `socks` feature。
- 登录使用共享 reqwest client 和全局 JWT token 维持会话，重新登录前和退出时调用 `clear_session`。
- 小说分组已过滤；详情接口遇到小说或书库内容会返回不支持提示，当前只实现漫画图片阅读链路。

## 阅读器行为

- 阅读页左右方向键翻页，`Esc` 返回，左右透明点击区也可翻页。
- 图片使用 `h-screen w-screen object-contain`，尽量占满视口但不裁切。
- 顶部 toolbar 鼠标移动时出现，短暂后隐藏；键盘操作时隐藏。
- 底部悬浮进度条显示页码；最后一页且存在下一章时显示“下一章”按钮。
- `pageIndex` search 作为初始页，`useReaderPages` 会规范化并在 manifest 返回页数后 clamp。
- 阅读进度只在已有章节标题且 `pageCount > 0` 时写入历史，避免保存无效页数。

## 阅读器当前实现

- `get_comic_read_manifest` 请求 `/chapter` 接口，用 `serde_json` 解析响应，构建图片页 manifest。
- `get_comic_read_page` 下载图片、必要时解扰、写入 app cache 的 `reader` 目录，再通过 Tauri asset protocol 给前端显示。
- `tauri.conf.json` 已允许 `$APPCACHE/reader/**`。
- 缓存上限默认约 512MB，设置页可选 128MB 到 2GB；修剪目标约为上限的 82%。
- 非扰图直接缓存原始 bytes，不再整图解码；冷缓存返回宽高 `0/0`，前端当前不依赖宽高排版。
- 扰图缓存输出 lossy WebP，当前使用 `webp` crate / libwebp，质量 `75.0`，缓存文件为 `.webp`。
- 扰图解码先 `image::load_from_memory`，转 RGB 后按 raw row copy 重排；主要成本通常在 `load_ms` 和 `encode_ms`。
- 前端阅读 hooks 已拆分：`use-reader-pages.ts` 负责组装，导航、manifest query、page query、prefetch 分别在独立 hook 中。
- 当前页请求传 `requestOrigin: 'visible'`；相邻预取页传 `requestOrigin: 'prefetch'`。
- 预取在 `use-reader-prefetch.ts` 中用 `queryClient.prefetchQuery` 串行执行，半径固定为 `BREEZE_ROW_PREFETCH_RADIUS = 1`，顺序为 `+1, -1`。
- 预取在当前页 ready 后触发；同一窗口用 `prefetchKeyRef` 去重。
- Rust 侧有同缓存路径 single-flight 锁，避免同一页重复冷处理。
- 缓存写入使用同目录 `.tmp` 文件，完成后 rename 为正式缓存文件；缓存统计和清理跳过 `.tmp`，避免并发读取半写文件。
- 开启性能调试日志时，阅读器日志包含 `origin=visible|prefetch`、`lock_wait_ms`、`download_ms`、`cache_ms`、`load_ms`、`reorder_ms`、`encode_ms`、`write_ms`、`cleanup_ms`、`source_kb`、`output_kb`、`ratio`、`delta_kb`。
- 当前没有单独的 Rust `prefetch_comic_read_pages` command，也没有设置页“预载数量/阅读线路”选项；如需恢复，先重新设计前后端契约。

## 当前优化路线图

原则：不碰测试基线和安全基线；不开放预取设置；不保留旧实现兼容分支；每轮完成后同步建议 `commit msg`。

### Phase 0：SQLite 存储基建

- 引入 Rust SQLite 层，优先使用 `sqlx` + SQLite，数据库只由 Rust 侧访问，前端继续走 Tauri `invoke`。
- 新建 `src-tauri/src/storage/`，负责连接池、schema 初始化和后续 migration。
- 数据库放应用数据目录，首批表包括 `download_tasks`、`download_chapters`、`reader_cache_entries`、`runtime_cache_entries`。
- 本阶段只建库和空表，不接入业务读写。
- 验证：`src-tauri` 下 `cargo check`。
- 建议 commit msg：`feat(storage): add sqlite storage foundation`。

### Phase 1：下载任务迁移到 SQLite

- 直接替换当前 JSON 下载任务持久化，不做旧 JSON 自动导入。
- `download_tasks` 存任务主信息；`download_chapters` 存章节列表和顺序。
- 前端 command 契约保持不变：新增、列表、暂停、恢复、取消、删除、打开目录仍走现有 invoke。
- 验证：`cargo check`，手动确认新任务能入库和恢复。
- 建议 commit msg：`refactor(downloads): persist tasks with sqlite`。

### Phase 2：下载进度写入节流

- 下载状态仍在内存实时更新，SQLite 进度写入按 500ms 或 1000ms 节流。
- 任务完成、失败、暂停、取消时强制 flush。
- 前端 1 秒轮询暂时不变。
- 验证：`cargo check`，手动下载多页章节确认进度更新。
- 建议 commit msg：`perf(downloads): throttle progress persistence`。

### Phase 3：Reader 缓存索引 SQLite 化

- 图片文件仍放 app cache 的 `reader` 目录，路径规则不变。
- `reader_cache_entries` 记录 endpoint、read_id、page_index、path、size_bytes、width、height、extension、is_scrambled、updated_at。
- 冷缓存写入成功后 upsert 索引；热缓存命中时更新 `updated_at`。
- 直接改用索引，不为旧缓存目录做一次性重建兼容。
- 验证：`cargo check`，冷缓存、热缓存、清缓存正常。
- 建议 commit msg：`perf(reader): index page cache metadata in sqlite`。

### Phase 4：Reader 缓存清理后台化

- 写入当前页后尽快返回，不在翻页关键路径递归扫描缓存目录。
- 超过缓存上限时触发单实例后台清理任务，按 SQLite 索引的 `updated_at` 删除旧文件。
- 清理失败只记录日志，不阻塞当前页显示。
- 验证：`cargo check`，调低缓存上限确认旧文件可清理。
- 建议 commit msg：`perf(reader): prune cache from sqlite index in background`。

### Phase 5：下载页有限并发

- 任务队列仍保持单任务执行，单个任务内部页面下载改为有限并发，默认 3 或 4。
- 保证输出目录和 `0001.ext` 命名不变。
- 暂停/取消需要尽快生效；任一页失败先让任务失败，后续再评估单页重试。
- 验证：`cargo check`，手动下载整章确认文件完整。
- 建议 commit msg：`perf(downloads): download pages with bounded concurrency`。

### Phase 6：Runtime API 缓存

- 使用 `runtime_cache_entries` 缓存短 TTL 数据：endpoint discovery、img_host、reader manifest。
- 第一轮 TTL 建议 endpoint/img_host 30 分钟，reader manifest 10-30 分钟。
- 网络失败时可返回未过期缓存并记录日志。
- 验证：`cargo check`，弱网/断网场景确认 fallback 行为。
- 建议 commit msg：`feat(api): cache runtime endpoint and reader metadata`。

### Phase 7：结构化错误返回

- Rust `ApiError` 增加可序列化错误 DTO：`kind`、`message`、`retryable`。
- Tauri command 使用统一 helper 返回结构化错误；前端 `tauriInvoke` 解析结构化错误。
- 登录过期、网络错误、API 失败、缓存失败分别显示准确文案。
- 验证：`cargo check`、`bun run build`。
- 建议 commit msg：`refactor(api): return structured command errors`。

### Phase 8：标准化日志与性能 span

- 引入 `tracing`、`tracing-subscriber`、`tracing-appender`，保留设置页调试日志开关。
- reader 日志统一 read_id、page、origin、cache_hit、download_ms、decode_ms、write_ms、cleanup_ms。
- download 日志统一 task_id、chapter_id、page、status、elapsed_ms。
- 旧 `diagnostics::*` 可先作为 wrapper 过渡，但不保留长期双实现。
- 验证：`cargo check`，设置页打开日志目录确认输出。
- 建议 commit msg：`refactor(logging): move diagnostics to tracing`。

### Phase 9：前端路由状态与 Query 策略收敛

- `list`、`ranking`、`weekly`、`search` 的分页和筛选统一进入 route search。
- 抽公共分页组件，减少重复。
- `QueryClient` 增加默认 retry、stale/gc 配置；reader query 单独覆盖。
- 不做 Query persistence，等 SQLite runtime cache 稳定后再评估。
- 验证：`bun run build`。
- 建议 commit msg：`refactor(frontend): normalize route state and query defaults`。

### Phase 10：文档同步与旧实现清理

- 更新本文档中的 SQLite 存储边界、reader cache index 行为、download 持久化和并发规则。
- 清理被替换的旧 JSON 下载持久化代码和不再使用的 helper。
- 验证：`cargo check`、`bun run build`。
- 建议 commit msg：`chore: document sqlite storage module boundaries`。

## 注意事项

- 历史观看依赖 `currentIndex`、`pageCount`、`chapterId`、`albumId` 等前端字段；优化图片解码和缓存时不要改 `get_comic_read_page` 的 `index` 语义、缓存路径命名规则或阅读页 search 参数。
- 阅读缓存路径规则目前是章节 read id 目录下 `0001.ext`；不要随意变更，否则热缓存命中和历史恢复体验会受影响。
- `src/features/reader/reader-bars.tsx` 可能有用户侧小改动，页码显示是 `{current} / {pageCount}`，不要无关回滚。
