export const isTauriRuntime = () => '__TAURI_INTERNALS__' in window

export async function invokeCommand<T>(command: string, args?: Record<string, unknown>) {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(command, args)
}
