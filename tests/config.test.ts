import { describe, expect, it } from 'vitest'
import { configSchema, defaultConfig } from '../src/shared/config'
import { actionSchema } from '../src/shared/actions'
import { displayMappingValid } from '../src/shared/workspace'

describe('desktop frontend configuration', () => {
  it('defaults saved pairings to remote start and preserves explicit opt-out', () => {
    const config = defaultConfig('desktop')
    const { allowRemoteStart: _, ...legacy } = config
    expect(configSchema.parse(legacy).allowRemoteStart).toBe(true)
    expect(configSchema.parse({ ...config, allowRemoteStart: false }).allowRemoteStart).toBe(false)
    expect(
      configSchema.safeParse({ ...config, deskflowPort: config.controlPort + 1 }).success,
    ).toBe(false)
  })

  it('starts with manual video switching and the 0.2 ports', () => {
    const config = defaultConfig('desktop')
    expect(configSchema.parse(config)).toEqual(config)
    expect(config.crossScreenVideo).toBe('manual')
    expect([config.controlPort, config.deskflowPort]).toEqual([49729, 24801])
    expect(config.shortcut).toBe('control+alt+F10')
    const { crossScreenVideo: _, ...withoutPolicy } = config
    expect(configSchema.parse(withoutPolicy).crossScreenVideo).toBe('manual')
  })

  it('does not share mutable defaults between drafts', () => {
    const first = defaultConfig('first')
    first.fallbackIps!.push('192.168.1.2')
    expect(defaultConfig('second').fallbackIps).toEqual([])
  })

  it('keeps legacy modifiers unchanged and accepts only supported mapping modes', () => {
    const config = defaultConfig('desktop')
    const { modifierMapping: _, ...legacy } = config
    expect(config.modifierMapping).toBe('original')
    expect(configSchema.parse(legacy).modifierMapping).toBe('original')
    const action = actionSchema.parse({
      type: 'save',
      config: { ...config, modifierMapping: 'common' },
    })
    expect(action.type === 'save' && action.config.modifierMapping).toBe('common')
    expect(configSchema.safeParse({ ...config, modifierMapping: 'invalid' }).success).toBe(false)
  })

  it('migrates mouse speed and validates saved receiver preferences', () => {
    const config = defaultConfig('receiver')
    const { receiverMouseSpeed: _, ...legacy } = config
    expect(configSchema.parse(legacy).receiverMouseSpeed).toBe(1)
    for (const receiverMouseSpeed of [0.5, 0.75, 1, 1.35, 2]) {
      const action = actionSchema.parse({ type: 'save', config: { ...config, receiverMouseSpeed } })
      expect(action.type === 'save' && action.config.receiverMouseSpeed).toBe(receiverMouseSpeed)
    }
    for (const receiverMouseSpeed of [0, 0.49, 2.01, -1, NaN, Infinity, '1']) {
      expect(configSchema.safeParse({ ...config, receiverMouseSpeed }).success).toBe(false)
    }
  })

  it('rejects conflicting ports and identical video inputs', () => {
    const config = defaultConfig('desktop')
    expect(configSchema.safeParse({ ...config, controlPort: config.deskflowPort }).success).toBe(
      false,
    )
    expect(configSchema.safeParse({ ...config, localInput: 1, peerInput: 1 }).success).toBe(false)
  })

  it('validates addresses and backup IPs before sending settings', () => {
    const config = defaultConfig('desktop')
    expect(configSchema.safeParse({ ...config, peerAddress: 'host\ninvalid' }).success).toBe(false)
    expect(configSchema.safeParse({ ...config, fallbackIps: ['not-an-ip'] }).success).toBe(false)
    expect(configSchema.safeParse({ ...config, fallbackIps: ['192.168.1.2', '::1'] }).success).toBe(
      true,
    )
  })

  it('accepts different input pairs for two distinct shared displays', () => {
    const primary = { ...defaultConfig('desktop'), displayId: 'left', localInput: 1, peerInput: 17 }
    const secondDisplay = { displayId: 'right', localInput: 18, peerInput: 3 }
    expect(displayMappingValid(primary)).toBe(true)
    const dual = { ...primary, secondDisplay }
    expect(displayMappingValid(dual)).toBe(true)
    expect(configSchema.parse(dual)).toEqual(dual)
    for (const invalid of [
      { ...secondDisplay, displayId: 'LEFT' },
      { ...secondDisplay, localInput: null },
      { ...secondDisplay, peerInput: secondDisplay.localInput },
    ]) {
      expect(displayMappingValid({ ...primary, secondDisplay: invalid })).toBe(false)
      expect(configSchema.safeParse({ ...primary, secondDisplay: invalid }).success).toBe(false)
    }
  })

  it('validates actions and rejects extra fields', () => {
    expect(actionSchema.parse({ type: 'switch', target: 'peer' })).toEqual({
      type: 'switch',
      target: 'peer',
    })
    for (const action of [
      { type: 'switch', target: 'unknown' },
      { type: 'testInput', value: -1 },
      { type: 'stop', command: 'unexpected' },
      { type: 'unknown' },
    ])
      expect(actionSchema.safeParse(action).success).toBe(false)
  })
})
