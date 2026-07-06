import { ImageIcon } from 'lucide-react'
import { useEffect, useState } from 'react'

import { cn } from '@/lib/utils'
import { useSettingsStore } from '@/stores/settings-store'

export function ComicCover({
  id,
  title,
  image,
  className,
  ratio = 'portrait',
  showIdBadge = false
}: {
  id?: string
  title: string
  image: string
  className?: string
  ratio?: 'portrait' | 'square'
  showIdBadge?: boolean
}) {
  const [hasImageError, setHasImageError] = useState(false)
  const shouldShowImage = image.length > 0 && !hasImageError

  useEffect(() => {
    setHasImageError(false)
  }, [image])

  return (
    <div
      className={cn(
        'relative overflow-hidden rounded-md bg-muted',
        ratio === 'square' ? 'aspect-square' : 'aspect-3/4',
        className
      )}
    >
      {shouldShowImage ? (
        <img
          src={image}
          alt={title}
          loading="lazy"
          decoding="async"
          referrerPolicy="no-referrer"
          className="h-full w-full object-cover"
          onError={() => setHasImageError(true)}
        />
      ) : (
        <CoverPlaceholder />
      )}
      {showIdBadge && id ? (
        <div className="absolute top-2 left-2 z-20 rounded-full border border-input/80 bg-background/45 px-2 py-1 text-[10px] backdrop-blur">
          JM {id}
        </div>
      ) : null}
      <CoverMask />
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
  const hideCovers = useSettingsStore(state => state.hideCovers)

  if (!hideCovers) {
    return null
  }

  return (
    <div className="absolute inset-0 z-10 flex items-center justify-center bg-muted/90 text-muted-foreground backdrop-blur-sm">
      <ImageIcon className="size-6" />
    </div>
  )
}
