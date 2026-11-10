import { reactive, watch } from 'vue'
import type { Config } from '../shared/types'
function copyConfig(config: Config): Config {
  return {
    ...config,
    ...(config.fallbackIps ? { fallbackIps: [...config.fallbackIps] } : {}),
    ...(config.secondDisplay ? { secondDisplay: { ...config.secondDisplay } } : {}),
  }
}
export function useConfigDraft(source: () => Config) {
  const draft = reactive<Config>(copyConfig(source()))
  let baseline = { ...source() }
  watch(
    () => ({ ...source() }),
    (incoming) => {
      // Keep edited fields; absorb unrelated external changes such as selecting
      // the engine executable. A successful save becomes the new baseline.
      const next = copyConfig(incoming)
      for (const key of Object.keys({ ...baseline, ...incoming }) as (keyof Config)[]) {
        if (JSON.stringify(draft[key]) === JSON.stringify(baseline[key])) {
          if (key in next) Object.assign(draft, { [key]: next[key] })
          else delete draft[key]
        }
      }
      baseline = { ...incoming }
    },
  )
  function value(): Config {
    return copyConfig(draft)
  }
  return { draft, value }
}
