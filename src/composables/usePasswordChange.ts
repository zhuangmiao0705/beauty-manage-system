import { reactive, ref } from 'vue'
import type { FormInstance, FormItemRule, FormRules } from 'element-plus'
import { changeOwnPassword } from '../auth'
import { errorMessage, notify } from '../utils/feedback'
import { validateForm } from '../utils/validation'

type RuleCallback = (error?: Error) => void

export function usePasswordChange(successMessage: string) {
  const saving = ref(false)
  const formRef = ref<FormInstance>()
  const form = reactive({ currentPassword: '', newPassword: '', confirmPassword: '' })
  const confirmPasswordRule: FormItemRule = {
    required: true,
    trigger: ['blur', 'change'],
    validator: (_rule: unknown, value: unknown, callback: RuleCallback) => {
      if (!value) callback(new Error('请再次输入新密码'))
      else if (value !== form.newPassword) callback(new Error('两次输入的密码不一致'))
      else callback()
    }
  }
  const rules: FormRules<typeof form> = {
    currentPassword: [{ required: true, message: '请输入当前密码', trigger: ['blur', 'change'] }],
    newPassword: [
      { required: true, message: '请输入新密码', trigger: ['blur', 'change'] },
      { min: 6, message: '新密码至少需要 6 位', trigger: ['blur', 'change'] }
    ],
    confirmPassword: [confirmPasswordRule]
  }

  async function submit() {
    if (!(await validateForm(formRef.value))) return false
    saving.value = true
    try {
      await changeOwnPassword(form.currentPassword, form.newPassword)
      formRef.value?.resetFields()
      notify(successMessage)
      return true
    } catch (reason) {
      notify(errorMessage(reason, '密码修改失败'), 'error')
      return false
    } finally {
      saving.value = false
    }
  }

  return { formRef, form, rules, saving, submit }
}
