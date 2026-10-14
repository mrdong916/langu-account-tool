const prefix = 'langu-search-history:';

export function normalizeHistory(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  return value
    .filter((term): term is string => typeof term === 'string')
    .map((term) => term.trim().replace(/\s+/g, ' '))
    .filter((term) => {
      const key = term.toLowerCase();
      if (!term || term.length > 100 || seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, 12);
}

export function readSearchHistory(scope: string): string[] {
  if (!scope) return [];
  try {
    return normalizeHistory(JSON.parse(localStorage.getItem(prefix + scope) ?? '[]'));
  } catch {
    return [];
  }
}

export function saveSearchHistory(scope: string, terms: string[]): string[] {
  if (!scope) return [];
  const history = normalizeHistory(terms);
  if (history.length) localStorage.setItem(prefix + scope, JSON.stringify(history));
  else localStorage.removeItem(prefix + scope);
  return history;
}
