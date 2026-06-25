import { useQuery } from '@tanstack/react-query'
import { createFileRoute, Link, useRouter } from '@tanstack/react-router'
import {
  ArrowLeftIcon,
  BookOpenIcon,
  BookmarkIcon,
  DownloadIcon,
  EyeIcon,
  HeartIcon,
  ImageIcon,
  LayersIcon,
  MessageCircleIcon,
  ThumbsUpIcon,
  UserRoundIcon,
  type LucideIcon
} from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

import { StatePanel } from '@/components/comic-feed'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Separator } from '@/components/ui/separator'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import {
  getComicComments,
  getComicDetail,
  type ComicChapter,
  type ComicComment,
  type ComicDetail,
  type RelatedComic
} from '@/lib/api/comic'
import { cn } from '@/lib/utils'

export const Route = createFileRoute('/_app/comic/$comicId')({
  component: ComicDetailPage
})

const DETAIL_STALE_TIME = 10 * 60 * 1000
const COMMENTS_STALE_TIME = 2 * 60 * 1000
const SHOW_COVER_MASK = true

function ComicDetailPage() {
  const { comicId } = Route.useParams()
  const router = useRouter()

  const detail = useQuery({
    queryKey: ['jm-comic-detail', comicId],
    queryFn: () => getComicDetail(comicId),
    staleTime: DETAIL_STALE_TIME
  })

  const comments = useQuery({
    queryKey: ['jm-comic-comments', comicId, 1],
    queryFn: () => getComicComments({ comicId }),
    staleTime: COMMENTS_STALE_TIME
  })

  return (
    <main className="min-h-screen bg-background p-[48px_32px_32px_96px] text-foreground">
      <div className="mx-auto max-w-7xl space-y-8">
        <Button variant="ghost" size="sm" onClick={() => router.history.back()}>
          <ArrowLeftIcon className="size-4" />
          返回
        </Button>

        {detail.isLoading ? (
          <ComicDetailSkeleton />
        ) : detail.isError ? (
          <StatePanel
            title="详情加载失败"
            description={detail.error.message}
            onRetry={() => detail.refetch()}
          />
        ) : detail.data == null ? (
          <StatePanel title="暂无详情" description="当前作品没有返回可展示的详情。" />
        ) : (
          <ComicDetailView
            comic={detail.data.comic}
            commentsState={{
              isLoading: comments.isLoading,
              isError: comments.isError,
              errorMessage: comments.error?.message,
              total: comments.data?.total ?? detail.data.comic.commentTotal,
              comments: comments.data?.comments ?? [],
              onRetry: () => comments.refetch()
            }}
          />
        )}
      </div>
    </main>
  )
}

function ComicDetailView({
  comic,
  commentsState
}: {
  comic: ComicDetail
  commentsState: CommentsState
}) {
  return (
    <div className="space-y-10">
      <ComicHero comic={comic} />

      <div className="grid grid-cols-[minmax(0,1fr)_320px] gap-8">
        <div className="min-w-0 space-y-8">
          <ChaptersSection chapters={comic.series} />
          <CommentsSection state={commentsState} />
        </div>

        <aside className="sticky top-8 h-fit">
          <RelatedPanel items={comic.relatedList} />
        </aside>
      </div>
    </div>
  )
}

function ComicHero({ comic }: { comic: ComicDetail }) {
  const authors = comic.author.length > 0 ? comic.author.join(' / ') : 'N/A'
  const statusBadges = [
    comic.price > 0 ? `${comic.price} 积分` : '免费',
    comic.purchased ? '已购买' : '',
    comic.isFavorite ? '已收藏' : '',
    comic.liked ? '已点赞' : ''
  ].filter(Boolean)

  return (
    <section className="grid grid-cols-[240px_minmax(0,1fr)] gap-8">
      <ComicCover id={comic.id} title={comic.title} image={comic.image} className="w-full" />

        <div className="min-w-0 space-y-5 py-1">
          <div className="flex flex-wrap items-center gap-2">
            {statusBadges.map(badge => (
              <Badge key={badge} variant="outline">
                {badge}
            </Badge>
          ))}
        </div>

        <div className="space-y-2">
          <h1 className="text-4xl leading-tight font-bold tracking-normal">{comic.title}</h1>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <UserRoundIcon className="size-4" />
            <span className="truncate">{authors}</span>
          </div>
        </div>

        <Separator />

        <StatsRow comic={comic} />

        <Separator />

        <p className="max-w-3xl text-sm leading-7 text-muted-foreground">
          {comic.description || '暂无简介'}
        </p>

        <div className="flex flex-wrap gap-2">
          <Button disabled>
            <BookOpenIcon className="size-4" />
            开始阅读
          </Button>
          <Button variant="outline" disabled>
            <BookmarkIcon className="size-4" />
            收藏
          </Button>
          <Button variant="outline" disabled>
            <DownloadIcon className="size-4" />
            下载
          </Button>
        </div>

        <div className="space-y-3">
          <PillGroup title="标签" items={comic.tags} />
          <PillGroup title="角色" items={comic.actors} variant="secondary" />
          <PillGroup title="作品" items={comic.works} variant="secondary" />
        </div>
      </div>
    </section>
  )
}

function StatsRow({ comic }: { comic: ComicDetail }) {
  const stats: Array<{ label: string; value: string; icon: LucideIcon }> = [
    { label: '浏览', value: formatNumber(comic.totalViews), icon: EyeIcon },
    { label: '喜欢', value: formatNumber(comic.likes), icon: HeartIcon },
    { label: '评论', value: formatNumber(comic.commentTotal), icon: MessageCircleIcon },
    { label: '章节', value: formatNumber(comic.series.length), icon: LayersIcon }
  ]

  return (
    <div className="flex items-stretch rounded-md bg-card/60 text-center text-sm">
      {stats.map((stat, index) => (
        <div key={stat.label} className="flex min-w-0 flex-1 items-center">
          <div className="flex min-w-0 flex-1 flex-col items-center justify-center space-y-1 p-4">
            <div className="flex items-center justify-center gap-2 text-xs font-medium text-muted-foreground">
              <stat.icon className="size-4" />
              {stat.label}
            </div>
            <div className="text-xl font-semibold">{stat.value}</div>
          </div>
          {index < stats.length - 1 ? <Separator orientation="vertical" /> : null}
        </div>
      ))}
    </div>
  )
}

function PillGroup({
  title,
  items,
  variant = 'outline'
}: {
  title: string
  items: string[]
  variant?: 'outline' | 'secondary'
}) {
  if (items.length === 0) {
    return null
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="w-10 text-xs text-muted-foreground">{title}</span>
      {items.map(item => (
        <Badge key={`${title}-${item}`} variant={variant}>
          {item}
        </Badge>
      ))}
    </div>
  )
}

function ChaptersSection({ chapters }: { chapters: ComicChapter[] }) {
  const sortedChapters = useMemo(() => sortChapters(chapters), [chapters])

  return (
    <section className="space-y-4">
      <SectionHeading title="章节" description={`${chapters.length} 个章节`} />
      {sortedChapters.length === 0 ? (
        <StatePanel title="暂无章节" description="当前作品没有返回章节列表。" />
      ) : (
        <div className="space-y-2">
          {sortedChapters.map((chapter, index) => (
            <Card key={chapter.id} size="sm" className="py-0 transition-colors hover:bg-muted/40">
              <CardContent className="flex items-center justify-between gap-4 p-4">
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">{chapter.title}</div>
                  <div className="text-xs text-muted-foreground">
                    {chapter.sort ? `第 ${chapter.sort} 章` : `章节 ${index + 1}`}
                  </div>
                </div>
                <Badge variant="outline">JM {chapter.id}</Badge>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </section>
  )
}

function CommentsSection({ state }: { state: CommentsState }) {
  return (
    <section className="space-y-4">
      <SectionHeading title="评论" description={`${formatNumber(state.total)} 条评论`} />
      {state.isLoading ? (
        <CommentSkeletonList />
      ) : state.isError ? (
        <StatePanel title="评论加载失败" description={state.errorMessage} onRetry={state.onRetry} />
      ) : state.comments.length === 0 ? (
        <StatePanel title="暂无评论" description="当前作品还没有返回评论内容。" />
      ) : (
        <div className="space-y-3">
          {state.comments.map(comment => (
            <CommentItem key={comment.id} comment={comment} />
          ))}
        </div>
      )}
    </section>
  )
}

function CommentItem({ comment }: { comment: ComicComment }) {
  const name = comment.nickname || comment.username || `用户 ${comment.userId}`
  const content = htmlToText(comment.content)

  return (
    <Card size="sm">
      <CardContent className="space-y-3 p-4">
        <div className="flex items-start gap-3">
          <CommentAvatar name={name} avatar={comment.avatar} />
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="truncate text-sm font-medium">{name}</span>
              {comment.spoiler ? <Badge variant="destructive">剧透</Badge> : null}
            </div>
            <div className="text-xs text-muted-foreground">{formatCommentTime(comment.time)}</div>
          </div>
        </div>

        <div className="space-y-3 pl-11">
          <p className="text-sm leading-6 text-card-foreground">{content || '这条评论没有内容'}</p>
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <ThumbsUpIcon className="size-3.5" />
            {formatNumber(comment.likeCount)}
          </div>

          {comment.replies.length > 0 ? (
            <div className="space-y-2 rounded-md bg-muted/60 p-3">
              {comment.replies.map(reply => (
                <ReplyItem key={reply.id} reply={reply} />
              ))}
            </div>
          ) : null}
        </div>
      </CardContent>
    </Card>
  )
}

function ReplyItem({ reply }: { reply: ComicComment }) {
  const name = reply.nickname || reply.username || `用户 ${reply.userId}`
  const content = htmlToText(reply.content)

  return (
    <div className="text-sm leading-6">
      <span className="font-medium">{name}</span>
      <span className="text-muted-foreground">：{content || '这条回复没有内容'}</span>
    </div>
  )
}

function CommentAvatar({ name, avatar }: { name: string; avatar: string }) {
  return (
    <Avatar size="lg">
      {avatar ? <AvatarImage src={avatar} alt={name} referrerPolicy="no-referrer" /> : null}
      <AvatarFallback>{getInitials(name)}</AvatarFallback>
    </Avatar>
  )
}

function RelatedPanel({ items }: { items: RelatedComic[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>相关推荐</CardTitle>
        <CardDescription>{items.length} 部作品</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {items.length === 0 ? (
          <p className="text-sm text-muted-foreground">暂无相关推荐</p>
        ) : (
          items.map(item => <RelatedItem key={item.id} item={item} />)
        )}
      </CardContent>
    </Card>
  )
}

function RelatedItem({ item }: { item: RelatedComic }) {
  return (
    <Link
      to="/comic/$comicId"
      params={{ comicId: item.id }}
      className="grid grid-cols-[64px_minmax(0,1fr)] gap-3 rounded-md p-1 transition-colors hover:bg-muted"
    >
      <ComicCover
        id={item.id}
        title={item.title}
        image={item.image}
        showId={false}
        className="w-16"
      />
      <div className="min-w-0 space-y-1 self-center">
        <Tooltip>
          <TooltipTrigger asChild>
            <div className="truncate text-sm font-medium">{item.title}</div>
          </TooltipTrigger>
          <TooltipContent>{item.title}</TooltipContent>
        </Tooltip>
        <div className="truncate text-xs text-muted-foreground">{item.author || 'N/A'}</div>
        <Badge variant="outline">JM {item.id}</Badge>
      </div>
    </Link>
  )
}

function ComicCover({
  id,
  title,
  image,
  showId = true,
  className
}: {
  id: string
  title: string
  image: string
  showId?: boolean
  className?: string
}) {
  const [hasImageError, setHasImageError] = useState(false)
  const shouldShowImage = image.length > 0 && !hasImageError

  useEffect(() => {
    setHasImageError(false)
  }, [image])

  return (
    <div
      className={cn(
        'relative aspect-[3/4] overflow-hidden rounded-md bg-muted ring-1 ring-border',
        className
      )}
    >
      {shouldShowImage ? (
        <img
          src={image}
          alt={title}
          loading="lazy"
          referrerPolicy="no-referrer"
          className="h-full w-full object-cover"
          onError={() => setHasImageError(true)}
        />
      ) : (
        <CoverPlaceholder />
      )}
      {SHOW_COVER_MASK ? <CoverMask /> : null}
      {showId ? (
        <div className="absolute top-2 left-2 z-20 rounded-full border border-input/80 bg-background/45 px-2 py-1 text-[10px] backdrop-blur">
          JM {id}
        </div>
      ) : null}
    </div>
  )
}

function CoverPlaceholder() {
  return (
    <div className="flex h-full items-center justify-center bg-muted text-muted-foreground">
      <ImageIcon className="size-6" />
    </div>
  )
}

function CoverMask() {
  return (
    <div className="absolute inset-0 z-10 flex items-center justify-center bg-muted/90 text-muted-foreground backdrop-blur-sm">
      <ImageIcon className="size-6" />
    </div>
  )
}

function SectionHeading({ title, description }: { title: string; description: string }) {
  return (
    <div className="flex items-end justify-between gap-4">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold tracking-normal">{title}</h2>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
    </div>
  )
}

function ComicDetailSkeleton() {
  return (
    <div className="space-y-10">
      <section className="grid grid-cols-[240px_minmax(0,1fr)] gap-8">
        <div className="aspect-[3/4] animate-pulse rounded-md bg-muted" />
        <div className="space-y-5 py-1">
          <div className="h-5 w-56 animate-pulse rounded bg-muted" />
          <div className="space-y-3">
            <div className="h-10 w-2/3 animate-pulse rounded bg-muted" />
            <div className="h-4 w-64 animate-pulse rounded bg-muted" />
          </div>
          <div className="h-px bg-border" />
          <div className="h-24 max-w-3xl animate-pulse rounded-md bg-muted" />
          <div className="h-px bg-border" />
          <div className="space-y-2">
            <div className="h-4 max-w-3xl animate-pulse rounded bg-muted" />
            <div className="h-4 max-w-2xl animate-pulse rounded bg-muted" />
            <div className="h-4 max-w-xl animate-pulse rounded bg-muted" />
          </div>
        </div>
      </section>
      <div className="grid grid-cols-[minmax(0,1fr)_320px] gap-8">
        <div className="space-y-8">
          <ChapterSkeletonList />
          <CommentSkeletonList />
        </div>
        <div className="h-80 animate-pulse rounded-xl bg-muted" />
      </div>
    </div>
  )
}

function ChapterSkeletonList() {
  return (
    <section className="space-y-4">
      <div className="space-y-2">
        <div className="h-6 w-24 animate-pulse rounded bg-muted" />
        <div className="h-4 w-32 animate-pulse rounded bg-muted" />
      </div>
      <div className="space-y-2">
        {Array.from({ length: 4 }).map((_, index) => (
          <div key={index} className="h-18 animate-pulse rounded-xl bg-muted" />
        ))}
      </div>
    </section>
  )
}

function CommentSkeletonList() {
  return (
    <div className="space-y-3">
      {Array.from({ length: 3 }).map((_, index) => (
        <Card key={index} size="sm">
          <CardContent className="space-y-3 p-4">
            <div className="flex items-start gap-3">
              <div className="size-10 animate-pulse rounded-full bg-muted" />
              <div className="flex-1 space-y-2">
                <div className="h-4 w-40 animate-pulse rounded bg-muted" />
                <div className="h-3 w-24 animate-pulse rounded bg-muted" />
              </div>
            </div>
            <div className="space-y-2 pl-11">
              <div className="h-4 animate-pulse rounded bg-muted" />
              <div className="h-4 w-2/3 animate-pulse rounded bg-muted" />
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

type CommentsState = {
  isLoading: boolean
  isError: boolean
  errorMessage?: string
  total: number
  comments: ComicComment[]
  onRetry: () => void
}

function sortChapters(chapters: ComicChapter[]) {
  return [...chapters].sort((left, right) => {
    const leftSort = Number.parseInt(left.sort, 10)
    const rightSort = Number.parseInt(right.sort, 10)

    if (Number.isNaN(leftSort) || Number.isNaN(rightSort)) {
      return 0
    }

    return rightSort - leftSort
  })
}

function formatNumber(value: number) {
  return new Intl.NumberFormat('zh-CN', {
    notation: value >= 10000 ? 'compact' : 'standard',
    maximumFractionDigits: 1
  }).format(value)
}

function formatCommentTime(value: string) {
  return value || '未知时间'
}

function htmlToText(value: string) {
  if (value.trim().length === 0) {
    return ''
  }

  const document = new DOMParser().parseFromString(value, 'text/html')

  return document.body.textContent?.trim() ?? value
}

function getInitials(value: string) {
  return value.trim().slice(0, 2).toUpperCase() || 'JM'
}
