export function parsePositivePage(value: unknown) {
  const page = Number.parseInt(String(value ?? ''), 10)

  return Number.isFinite(page) && page > 0 ? page : 1
}

export function parseStringSearch(value: unknown, fallback = '') {
  return typeof value === 'string' ? value : fallback
}
