import { createSSRApp, h } from 'vue'
import { renderToString } from 'vue/server-renderer'
import { describe, expect, it } from 'vitest'
import PointerSettings from '../src/components/PointerSettings.vue'
import SharingPanel from '../src/components/SharingPanel.vue'
import { defaultConfig } from '../src/shared/config'
import type { Snapshot } from '../src/shared/types'

function state(role: 'host' | 'client', running = false): Snapshot {
  return {
    config: { ...defaultConfig('Mac'), role, receiverMouseSpeed: 0.75 },
    platform: 'darwin',
    cursorDisplays: [],
    connected: false,
    running,
    sessionLayout: null,
    deviceId: 'mac',
    addresses: [],
    monitors: [],
    listening: false,
    engine: 'stopped',
    peer: null,
    activeDeviceId: null,
    displayDeviceId: null,
    switching: false,
    pairingCode: '',
    hasSavedPairing: false,
    dependencies: { deskflow: true, displayControl: true, accessibility: true },
    diagnostics: { directory: '', error: null },
    logs: [],
    lastError: null,
    preview: false,
  }
}

async function render(role: 'host' | 'client', locked = false) {
  return renderToString(
    createSSRApp(() =>
      h(PointerSettings, {
        state: state(role),
        isHost: role === 'host',
        locked,
        side: 'off',
        video: 'manual',
        retained: '',
        speed: 0.75,
      }),
    ),
  )
}

describe('receiver mouse speed', () => {
  it('shows the saved local multiplier only on the receiver and locks during sharing', async () => {
    const html = await render('client')
    expect(html).toContain('本机接收鼠标速度 · 0.75×')
    expect(html).toMatch(/<input[^>]*type="range"[^>]*min="0.5"[^>]*max="2"/)
    expect(html).not.toMatch(/<input[^>]*disabled/)
    expect(await render('client', true)).toMatch(/<input[^>]*disabled/)
    expect(await render('host')).not.toContain('本机接收鼠标速度')
  })

  it('wires the saved preference through the sharing settings draft', async () => {
    const html = await renderToString(
      createSSRApp(() =>
        h(SharingPanel, {
          state: state('client'),
          busy: false,
          checkingAccessibility: false,
        }),
      ),
    )
    expect(html).toContain('本机接收鼠标速度 · 0.75×')
    expect(html).toContain('保存并重新开始共享后生效')
  })
})
