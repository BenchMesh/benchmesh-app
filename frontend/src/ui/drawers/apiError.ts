/** The reason an API error response gives (`detail`), else its status. */
export async function apiError(resp: Response): Promise<string> {
  const data = await resp.json().catch(() => null);
  const d = data?.detail;
  if (typeof d === 'string') return d;
  if (d != null) return JSON.stringify(d);
  return `HTTP ${resp.status}`;
}
