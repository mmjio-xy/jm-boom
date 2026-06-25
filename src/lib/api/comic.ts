import { invoke } from '@tauri-apps/api/core'

export type RelatedComic = {
  id: string
  title: string
  author: string
  image: string
}

export type ComicChapter = {
  id: string
  title: string
  sort: string
}

export type ComicDetail = {
  id: string
  title: string
  author: string[]
  description: string
  totalViews: number
  likes: number
  commentTotal: number
  tags: string[]
  actors: string[]
  works: string[]
  isFavorite: boolean
  liked: boolean
  relatedList: RelatedComic[]
  series: ComicChapter[]
  seriesId: string
  price: number
  purchased: boolean
  image: string
}

export type ComicDetailResult = {
  endpoint: string
  comic: ComicDetail
}

export type ComicComment = {
  id: string
  comicId?: string | null
  userId: string
  username: string
  nickname: string
  content: string
  likeCount: number
  time: string
  updatedAt: string
  avatar: string
  parentId: string
  spoiler: boolean
  replies: ComicComment[]
}

export type ComicCommentsResult = {
  endpoint: string
  page: number
  total: number
  comments: ComicComment[]
}

export async function getComicDetail(
  comicId: string,
  endpoint: string | null = null
): Promise<ComicDetailResult> {
  ensureTauriRuntime()

  return invoke<ComicDetailResult>('get_comic_detail', {
    comicId,
    endpoint
  })
}

export async function getComicComments({
  comicId,
  page = 1,
  endpoint = null
}: {
  comicId: string
  page?: number
  endpoint?: string | null
}): Promise<ComicCommentsResult> {
  ensureTauriRuntime()

  return invoke<ComicCommentsResult>('get_comic_comments', {
    comicId,
    page,
    endpoint
  })
}

function ensureTauriRuntime() {
  if (!('__TAURI_INTERNALS__' in window)) {
    throw new Error('This content needs the Tauri desktop runtime.')
  }
}
