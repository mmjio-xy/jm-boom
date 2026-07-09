interface EmptyStateProps {
  emoji: string
  title: string
}

export function EmptyState({ emoji, title }: EmptyStateProps) {
  return (
    <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-4 text-center">
      <p className="text-6xl font-bold">{emoji}</p>
      <p className="text-sm text-muted-foreground">{title}</p>
    </div>
  )
}
