import type { PackageDefinition, PackagePurchase } from '../types'
import { fullDateTime } from './formatters'

export function isPackagePurchaseAvailable(purchase: PackagePurchase, at = new Date()) {
  if (purchase.status !== 'active' || purchase.refundedAt) return false
  if (purchase.limitType === 'time') {
    if (!purchase.expiresAt) return false
    return new Date(purchase.expiresAt).getTime() > at.getTime()
  }
  return purchase.remainingUses > 0
}

export function packageDefinitionLimitText(item: PackageDefinition) {
  return item.limitType === 'time' ? `${item.validityDays}天` : `${item.totalUses}次`
}

export function packagePurchaseLimitText(item: PackagePurchase) {
  if (item.refundedAt) return `已退款 ${item.refundAmount.toFixed(2)} 元`
  return item.limitType === 'time'
    ? `有效至 ${item.expiresAt ? fullDateTime(item.expiresAt) : '--'}`
    : `剩余 ${item.remainingUses}/${item.totalUses} 次`
}
