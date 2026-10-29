/**
 * Instruments - About: this BenchMesh - which build it is, its version (the official build only: a
 * community build shows "v<version> community" in the title bar and nowhere else), documentation
 * and releases - and the groups the distribution adds (`extensions.aboutGroups`: the official
 * build's Updates, which may stop BenchMesh to install).
 */
import { PageLink } from './links'
import { useAppVersion } from './version'
import { useViewOnly } from '../remote/RemoteProvider'
import { ComebackNotice, useComeback } from '../system/comeback'
import { extensions } from '../../extensions'

export function AboutTab() {
  const viewOnly = useViewOnly()
  const version = useAppVersion()
  const comeback = useComeback()
  return (
    <section className="bench-page" aria-label="About" data-testid="about-tab">
      <h3 className="settings-view__section-title">About</h3>
      {comeback.phase !== 'running' ? <ComebackNotice phase={comeback.phase} stopping={comeback.stopping} /> : (
        <>
          <div className="bench-group" role="group" aria-label="BenchMesh">
            <span className="kit-label">{version?.official ? `BenchMesh ${version.version}` : 'BenchMesh'}</span>
            <p className="bench-hint">One cockpit for lab instruments: control them, record them on one timeline and automate them.</p>
            {version && (version.official
              ? <p className="bench-hint">The official build, signed by BenchMesh.</p>
              : <p className="bench-hint">A community build, made from the open-source code (MIT license). It does not update itself: to update, build the new version.</p>)}
            <div className="bench-group__row">
              <PageLink page="docs">Documentation</PageLink>
              <PageLink page="releases">Releases</PageLink>
            </div>
          </div>
          {extensions.aboutGroups.map((Group, i) => <Group key={i} viewOnly={viewOnly} onStopping={comeback.onStopping} />)}
        </>
      )}
    </section>
  )
}
