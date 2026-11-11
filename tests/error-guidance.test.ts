import { describe, expect, it } from 'vitest'
import { errorGuidance } from '../src/shared/error-guidance'

const invocation = `恢复显示输出失败: 原生调用失败：mac-output restore ${JSON.stringify({
  method: 'mac-native-v1',
  panels: [{ uuid: 'external', width: 1920, height: 1080, x: 0, y: 0 }],
  target: { uuid: 'external', width: 1920, height: 1080, x: -1920, y: 0 },
})}；退出状态=exit status: 1`

describe('native error guidance', () => {
  it('shows the failure after a long baseline while retaining the recovery context', () => {
    const failure = {
      action: 'save' as const,
      message: `${invocation}；stderr=提交显示输出事务失败：1001；stdout=未提供输出`,
    }
    const guidance = errorGuidance(failure)
    expect(guidance.summary).toBe('恢复显示输出失败: 提交显示输出事务失败：1001')
    expect(guidance.destination).toBe('display')
    expect(failure.message).toContain('mac-native-v1')
  })

  it('uses stdout diagnostics when stderr is empty', () => {
    const guidance = errorGuidance({
      action: 'testInput',
      message: `${invocation}；stderr=未提供输出；stdout=DDC bus unavailable\nmore details`,
    })
    expect(guidance.summary).toBe('恢复显示输出失败: DDC bus unavailable')
  })

  it('preserves ordinary summaries and bounds native diagnostics', () => {
    expect(errorGuidance({ action: 'save', message: '请选择外接屏\n详细信息' }).summary).toBe(
      '请选择外接屏',
    )
    const message = `${invocation}；stderr=${'错'.repeat(700)}；stdout=未提供输出`
    expect(errorGuidance({ action: 'save', message }).summary).toHaveLength(220)
    expect(errorGuidance({ action: 'save', message: '' }).summary).toContain('未收到错误详情')
  })

  it('retains invocation diagnostics when neither output stream has a reason', () => {
    const message = `${invocation}；stderr=未提供输出；stdout=未提供输出`
    expect(errorGuidance({ action: 'save', message }).summary).toBe(message.slice(0, 220))
  })
})
