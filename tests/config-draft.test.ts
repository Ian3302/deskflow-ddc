import { effectScope, nextTick, shallowRef } from 'vue'
import { expect, it } from 'vitest'
import { useConfigDraft } from '../src/composables/useConfigDraft'
import { defaultConfig } from '../src/shared/config'
import type { Config } from '../src/shared/types'
it('accepts imported IP fallback changes while preserving an unsaved name edit', async () => {
  const config = shallowRef({ ...defaultConfig('before'), fallbackIps: ['192.168.1.2'] })
  const scope = effectScope()
  const draft = scope.run(() => useConfigDraft(() => config.value))!
  draft.draft.deviceName = 'unsaved'
  config.value = { ...config.value, fallbackIps: ['192.168.1.3'], deskflowPath: '/core' }
  await nextTick()
  expect(draft.draft.deviceName).toBe('unsaved')
  expect(draft.value().fallbackIps).toEqual(['192.168.1.3'])
  expect(draft.draft.deskflowPath).toBe('/core')
  draft.value().fallbackIps!.push('192.168.1.4')
  expect(draft.draft.fallbackIps).toEqual(['192.168.1.3'])
  scope.stop()
})

it('keeps an edited second display separate from snapshots and accepts its removal', async () => {
  const secondDisplay = { displayId: 'right', localInput: 18, peerInput: 3 }
  const config = shallowRef<Config>({ ...defaultConfig('desktop'), secondDisplay })
  const scope = effectScope()
  const { draft, value } = scope.run(() => useConfigDraft(() => config.value))!
  draft.secondDisplay!.peerInput = 4
  expect(config.value.secondDisplay!.peerInput).toBe(3)
  const saved = value()
  config.value = saved
  await nextTick()
  expect(draft.secondDisplay!.peerInput).toBe(4)
  config.value = defaultConfig('desktop')
  await nextTick()
  expect(draft.secondDisplay).toBeUndefined()
  scope.stop()
})
