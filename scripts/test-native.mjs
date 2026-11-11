import { execFileSync, spawnSync } from 'node:child_process'
import { mkdirSync, readFileSync } from 'node:fs'
// Windows PowerShell 5.1 reads BOM-less scripts using the system code page.
const windowsBridge = readFileSync('native/windows/monitor.ps1')
if (
  windowsBridge.some((byte) => byte > 127) &&
  windowsBridge.subarray(0, 3).toString('hex') !== 'efbbbf'
)
  throw new Error('Windows bridge contains non-ASCII text but is missing a UTF-8 BOM.')
mkdirSync('.native-cache', { recursive: true })
execFileSync(
  'clang',
  [
    '-std=c11',
    '-Wall',
    '-Wextra',
    '-Iresources/vendor/m1ddc/headers',
    'native/tests/ddc-reply.c',
    '-o',
    '.native-cache/test-ddc',
  ],
  { stdio: 'inherit' },
)
execFileSync('.native-cache/test-ddc', [], { stdio: 'inherit' })
if (process.platform === 'darwin') {
  execFileSync(
    'swiftc',
    [
      '-module-cache-path',
      '.native-cache',
      'native/mac-hotkey.swift',
      '-o',
      '.native-cache/test-mac-hotkey',
    ],
    { stdio: 'inherit' },
  )
  execFileSync('.native-cache/test-mac-hotkey', ['self-test'], { stdio: 'inherit' })
  // This action only inspects state; it never posts keyboard or mouse input.
  const inputStatus = JSON.parse(
    execFileSync('.native-cache/test-mac-hotkey', ['diagnose'], { encoding: 'utf8' }),
  )
  for (const field of ['accessibility', 'postEventAccess', 'secureEventInput']) {
    if (typeof inputStatus[field] !== 'boolean')
      throw new Error(`Missing native input diagnostic field: ${field}`)
  }
  for (const source of ['hid', 'session']) {
    for (const modifier of ['control', 'option', 'shift', 'command']) {
      for (const side of ['left', 'right']) {
        if (typeof inputStatus.modifiers?.[source]?.[modifier]?.[side] !== 'boolean')
          throw new Error(`Missing native modifier state: ${source}.${modifier}.${side}`)
      }
      if (typeof inputStatus.modifiers?.[source]?.flags?.[modifier] !== 'boolean')
        throw new Error(`Missing native modifier flag: ${source}.${modifier}`)
    }
  }
  const snapshots = execFileSync('.native-cache/test-mac-hotkey', ['watch-modifiers', '1'], {
    encoding: 'utf8',
    timeout: 5000,
  })
    .trim()
    .split('\n')
    .map((line) => JSON.parse(line))
  if (
    !snapshots.length ||
    snapshots.some((sample) => !sample.modifiers || !Number.isFinite(sample.time))
  )
    throw new Error('Modifier watcher must return timestamped snapshots')
  for (const invalid of ['0', '61', 'invalid']) {
    const result = spawnSync('.native-cache/test-mac-hotkey', ['watch-modifiers', invalid], {
      timeout: 5000,
    })
    if (result.status !== 2)
      throw new Error(`Modifier watcher accepted invalid duration: ${invalid}`)
  }
  execFileSync(
    'swiftc',
    [
      '-module-cache-path',
      '.native-cache',
      'native/mac-output.swift',
      '-o',
      '.native-cache/test-mac-output',
    ],
    { stdio: 'inherit' },
  )
  execFileSync('.native-cache/test-mac-output', ['self-test'], { stdio: 'inherit' })
  execFileSync(
    'clang',
    [
      '-fobjc-arc',
      '-Wall',
      '-Wextra',
      '-framework',
      'AppKit',
      'native/tests/mac-lifecycle.m',
      '-o',
      '.native-cache/test-mac-lifecycle',
    ],
    { stdio: 'inherit' },
  )
  execFileSync('.native-cache/test-mac-lifecycle', [], { stdio: 'inherit' })
}
