# JM Boom Project Notes

本文件记录当前项目已经实现的关键工作和后续接手约定，避免下次继续开发时重新摸索。

## 基本约定

- 这是 Tauri 2 + React 19 + Vite + TanStack Router/Query + shadcn/ui + Rust reqwest 项目。
- 当前优先桌面端体验，暂时不重点处理小屏适配。
- 用户明确说过：代码已格式化，不用优先管 lint。常规验证优先跑 `bun run build`；改 Rust 后再跑 `cargo check`。
- 不要手动编辑 `src/routeTree.gen.ts`。路由变更后由 TanStack Router 插件、HMR 或 build 自动生成。
- 前端和 Rust 通信用 Tauri `invoke`，新接口不要再新增 axios 调用。`axios` 依赖目前仍在 `package.json`，但新代码默认不依赖它。
- UI 优先复用 shadcn/ui 已安装组件。需要新 shadcn 组件时，告诉用户安装，不要自己重造复杂基础组件。
- 项目已有 `floating-nav`，不要再加传统 sidebar。
- 需要安装外部依赖，如果该依赖是 Rust 需要的，则直接添加，后续使用 `cargo check` 下载，如果是前端项目，则直接告诉用户，由用户来安装相关依赖。
- 项目接口实现或数据来源参考同级项目的 `jm-mobile` 项目。

## 已实现页面

- 首页：`src/routes/_app/index.tsx`
  - 通过 `getHomeFeed` 获取信息流。
  - 从上到下依次渲染各个信息流大项，不使用 tabs。
  - 右侧有轻量“导航”目录，可平滑跳转并随滚动高亮当前分组。
  - 有独立 BackTop 组件，滚动到一定距离后显示。

- 每周推荐：`src/routes/_app/weekly.tsx`
  - 已使用 shadcn `Tabs` 和 `Select`。
  - filters 使用较长 stale time，切换 tabs/select 时只刷新下方 items 区域。
  - items 使用 `ComicGrid`，骨架屏尺寸和真实 card 保持一致。

- 详情页：`src/routes/_app/comic/$comicId.tsx`
  - 展示封面、标题、作者、stats、简介、标签、章节、相关推荐。
  - 评论通过 stats 中的评论入口打开右侧 Drawer。
  - 评论 Drawer 不显示头像，滚动到底部自动加载更多；无更多时显示“暂无更多评论”。
  - 评论时间使用 `date-fns` 中文相对时间；大于 7 天显示具体时间。
  - 章节使用 shadcn `Pagination`，每页 10 个。
  - 章节标题兜底规则：有 `chapter.title` 用标题；否则有 `sort` 显示 `第 N 章`；再否则显示 `章节 N`。
  - 章节进入阅读页时会传 `title`、`chapter`、`albumId`、`nextId`、`nextChapter`。

- 阅读页：`src/routes/reader/$comicId.tsx`
  - 独立于 `_app`，所以没有 floating-nav。
  - 顶部 toolbar 显示返回、作品标题、章节名、刷新按钮；不显示页码。
  - toolbar 在鼠标移动时出现，短暂后隐藏；键盘操作时隐藏。
  - 左右方向键翻页，`Esc` 返回；页面左右透明点击区也可翻页。
  - 图片使用 `h-screen w-screen object-contain`，尽量占满视口但保留原比例，不裁切。
  - 底部 footer 是悬浮预览组件：`absolute bottom-20 left-1/2 w-80`，圆角背景，内部上方进度条、下方页码。
  - 到当前章节最后一页且存在下一章时，右下角单独显示“下一章”按钮。
  - 阅读页会优先使用 search 中的章节名；如果为空，会根据 `albumId` 获取详情并按同一兜底规则生成章节名。

## 组件与视觉状态

- `src/components/comic-feed.tsx`
  - `ComicGrid` 为 4 列网格。
  - Comic card 使用 shadcn `Card`。
  - 封面是 `aspect-square`，只展示标题和作者。
  - 标题单行省略，hover 使用项目 Tooltip。
  - 封面左上角半透明圆角容器显示 `JM id`。
  - 当前 `SHOW_COVER_MASK = true`，真实图上覆盖蒙版；需要展示真实图片时删除或关闭该蒙版即可。

## Rust / Tauri API

- Rust API 已从 `lib.rs` 拆出：
  - `src-tauri/src/api.rs`：远程设置、搜索、首页、每周推荐、详情、评论等普通接口。
  - `src-tauri/src/reader.rs`：阅读 manifest、图片下载、解码、缓存、预取。
  - `src-tauri/src/lib.rs`：只保留 Tauri command 包装和 invoke handler 注册。

- 已注册 Tauri commands：
  - `get_remote_setting`
  - `search_comics`
  - `get_home_feed`
  - `get_week_filters`
  - `get_week_items`
  - `get_comic_detail`
  - `get_comic_comments`
  - `get_comic_read_manifest`
  - `get_comic_read_page`
  - `prefetch_comic_read_pages`

- 阅读器 Rust 侧要点：
  - 读取 `chapter_view_template`。
  - 使用 `json5` 解析源端宽松 JS/JSON 对象。
  - 使用 `image` crate 解码被切割/扰乱的图片。
  - 解码后页面缓存到 app cache 的 `reader` 目录。
  - 缓存上限约 512MB，修剪目标约 420MB。
  - 当前页会预取前后约 3 页。
  - `tauri.conf.json` 已开启 asset protocol，并允许 `$APPCACHE/reader/**`。

## 验证建议

- 仅前端改动：跑 `bun run build`。
- 改 `src-tauri` Rust 代码：跑 `cargo check`，工作目录为 `src-tauri`。
- 用户此前表示暂时不要纠结 lint；不要因为 lint 结果扩大无关改动。

