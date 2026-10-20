/** Short texts for remote sessions: the browser from its user agent, the sign-in time. */

const BROWSERS: [RegExp, string][] = [
  [/Edg(e|A|iOS)?\//, 'Edge'],
  [/OPR\/|Opera/, 'Opera'],
  [/Firefox\/|FxiOS\//, 'Firefox'],
  [/Chrome\/|CriOS\//, 'Chrome'],
  [/Version\/[\d.]+.*Safari\//, 'Safari'],
]

const SYSTEMS: [RegExp, string][] = [
  [/Windows/, 'Windows'],
  [/iPhone|iPad|iPod/, 'iOS'],
  [/Mac OS X|Macintosh/, 'macOS'],
  [/Android/, 'Android'],
  [/CrOS/, 'ChromeOS'],
  [/Linux/, 'Linux'],
]

/** "Chrome on Windows"; an unknown agent is shortened to 40 characters. */
export function shortAgent(agent: string | null | undefined): string {
  const ua = (agent ?? '').trim()
  if (!ua) return 'Unknown browser'
  const browser = BROWSERS.find(([re]) => re.test(ua))?.[1]
  const system = SYSTEMS.find(([re]) => re.test(ua))?.[1]
  if (browser && system) return `${browser} on ${system}`
  if (browser || system) return (browser ?? system)!
  return ua.length > 40 ? `${ua.slice(0, 39)}…` : ua
}

/** Time of day for today, date and time otherwise. */
export function sinceText(ms: number, now = Date.now()): string {
  const d = new Date(ms)
  if (!Number.isFinite(ms)) return '?'
  const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  return new Date(now).toDateString() === d.toDateString() ? time : `${d.toLocaleDateString()} ${time}`
}
