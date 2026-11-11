import type { ActionFailure } from './workspace'
function errorSummary(message: string): string {
  const nativeCall = message.indexOf('原生调用失败：')
  if (nativeCall >= 0) {
    const stderr = message.match(/；stderr=([\s\S]*?)；stdout=/)?.[1]?.trim()
    const stdout = message.match(/；stdout=([\s\S]*)$/)?.[1]?.trim()
    const detail = [stderr, stdout].find((value) => value && value !== '未提供输出')
    if (detail) return `${message.slice(0, nativeCall)}${detail.split(/\r?\n/)[0]}`.slice(0, 220)
  }
  return message.split(/\r?\n/)[0]?.slice(0, 220) || '未收到错误详情，请导出诊断日志。'
}

export function errorGuidance(failure: ActionFailure) {
  const message = failure.message
  const summary = errorSummary(message)
  if (['switch', 'testInput'].includes(failure.action) && /第[一二]台显示器[：指]/.test(message))
    return {
      summary,
      advice:
        failure.action === 'switch'
          ? '检查失败屏幕的输入源映射和连接，再用主界面明确选择目标电脑重试。'
          : '检查失败屏幕的输入源映射和连接，再重试两个方向。',
      destination: 'display',
      label: '检查显示器配置',
    }
  if (/辅助功能|权限|accessibility/i.test(message))
    return {
      summary,
      advice: '在系统设置中检查辅助功能权限，返回后重新检测；共享连接需保持就绪。',
      destination: 'sharing',
      label: '检查共享权限',
    }
  if (/恢复.*输出|停用原输出|保留输出|guardian|原生调用/i.test(message))
    return {
      summary,
      advice:
        '画面未恢复时先停止共享；若提示本机输出待恢复，可从主界面或托盘选择“恢复本机显示输出”重试。必要时用显示器实体按键选回输入。',
      destination: 'display',
      label: '检查显示器配置',
    }
  if (/failed to connect to server|connection refused|server is not responding/i.test(message))
    return {
      summary: '键鼠引擎无法连接提供端。',
      advice:
        '确认提供端正在共享，并检查两端的键鼠共享端口和防火墙。连接重试期间可在工作台查看引擎状态。',
      destination: 'sharing',
      label: '检查共享设置',
    }
  if (/键鼠|Deskflow|引擎/i.test(message))
    return {
      summary,
      advice: '检查两端引擎是否就绪及提供端权限；无法恢复控制时，先停止共享并恢复输出。',
      destination: 'sharing',
      label: '检查共享设置',
    }
  if (/连接|对端|会话|网络|主机|配对|端口|TCP/i.test(message))
    return {
      summary,
      advice: '确认另一端正在运行、两端位于同一局域网，检查地址及配对状态后重新连接。',
      destination: 'pairing',
      label: '检查设备连接',
    }
  if (
    ['switch', 'testInput', 'scan', 'scanDisplays', 'checkOutput', 'restoreOutput'].includes(
      failure.action,
    )
  )
    return {
      summary,
      advice: '检查输入源映射、显示器菜单和视频连接。读取失败不能直接判定显示器不支持切换。',
      destination: 'display',
      label: '检查显示器配置',
    }
  return {
    summary,
    advice: '检查相关设置；仍无法完成时，请导出诊断日志并保留操作步骤。',
    destination: 'activity',
    label: '查看活动记录',
  }
}
