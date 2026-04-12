/** Multi-term fuzzy search: every whitespace-separated term must appear in at least one field. */
export function matchesQuery(query: string, ...fields: (string | undefined)[]): boolean {
  const terms = query.split(/\s+/).filter(Boolean);
  if (terms.length === 0) return false;
  return terms.every((term) => fields.some((field) => field?.toLowerCase().includes(term)));
}
