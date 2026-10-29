/**
 * Instruments - Online: the services of benchmesh.online a distribution adds
 * (`extensions.onlineGroups`: the official build's BenchMesh account, problem reports and
 * confirmations). Shown only when there are any; the top bar's status (`extensions.topBarStatus`)
 * opens it.
 */
import { useViewOnly } from '../remote/RemoteProvider'
import { ComebackNotice, useComeback } from '../system/comeback'
import { extensions } from '../../extensions'

export const hasOnlineTab = () => extensions.onlineGroups.length > 0

export function OnlineTab() {
  const viewOnly = useViewOnly()
  const comeback = useComeback()
  return (
    <section className="bench-page" aria-label="Online" data-testid="online-tab">
      <h3 className="settings-view__section-title">Online</h3>
      {comeback.phase !== 'running'
        ? <ComebackNotice phase={comeback.phase} stopping={comeback.stopping} />
        : extensions.onlineGroups.map((Group, i) => <Group key={i} viewOnly={viewOnly} onStopping={comeback.onStopping} />)}
    </section>
  )
}
