import type { FormInstance, FormItemRule } from 'element-plus'

type RuleCallback = (error?: Error) => void

export async function validateForm(form: FormInstance | null | undefined) {
  if (!form) return false
  try {
    await form.validate()
    return true
  } catch {
    return false
  }
}

export function requiredTextRule(message: string): FormItemRule {
  return {
    required: true,
    trigger: ['blur', 'change'],
    validator: (_rule: unknown, value: unknown, callback: RuleCallback) => {
      if (typeof value === 'string' && value.trim()) callback()
      else callback(new Error(message))
    }
  }
}

export function positiveNumberRule(message: string): FormItemRule {
  return {
    required: true,
    trigger: ['blur', 'change'],
    validator: (_rule: unknown, value: unknown, callback: RuleCallback) => {
      if (typeof value === 'number' && Number.isFinite(value) && value > 0) callback()
      else callback(new Error(message))
    }
  }
}

export function nonNegativeNumberRule(message: string): FormItemRule {
  return {
    required: true,
    trigger: ['blur', 'change'],
    validator: (_rule: unknown, value: unknown, callback: RuleCallback) => {
      if (typeof value === 'number' && Number.isFinite(value) && value >= 0) callback()
      else callback(new Error(message))
    }
  }
}
