import { ref, computed } from 'vue'

export type PipelineStatus = 'idle' | 'busy' | 'error'

export function usePipelineStatus() {
  const status = ref<PipelineStatus>('idle')
  const label = ref('')

  const isBusy = computed(() => status.value === 'busy')
  const isError = computed(() => status.value === 'error')
  const isIdle = computed(() => status.value === 'idle')

  const setStatus = (newStatus: PipelineStatus, newLabel = '') => {
    status.value = newStatus
    label.value = newLabel
  }

  const setBusy = (newLabel = '处理中...') => setStatus('busy', newLabel)
  const setIdle = () => setStatus('idle')
  const setError = (errorLabel = '错误') => setStatus('error', errorLabel)

  return {
    status: readonly(status),
    label: readonly(label),
    isBusy,
    isError,
    isIdle,
    setStatus,
    setBusy,
    setIdle,
    setError
  }
}