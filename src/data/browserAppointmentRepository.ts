import { COMMISSION_RULE_VERSION } from '../config/options'
import type {
  AppSnapshot,
  AppointmentCompletionInput,
  AppointmentInput,
  AppointmentStatus
} from '../types'

const roundMoney = (value: number) => Math.round(value * 100) / 100

function resolveAppointment(snapshot: AppSnapshot, input: AppointmentInput) {
  const startsAt = new Date(input.startsAt)
  if (Number.isNaN(startsAt.getTime())) throw new Error('预约时间格式无效')
  if (input.duration <= 0 || input.duration > 1440) throw new Error('预约时长必须在1至1440分钟之间')
  const employee = snapshot.employees.find(
    item => item.name === input.employee && item.status === 'active'
  )
  if (!employee) throw new Error('没有找到启用状态的员工')

  let memberId: string | null = null
  let customerName = input.guestName.trim()
  let customerPhone = input.guestPhone.trim()
  if (input.customerType === 'member') {
    const member = snapshot.members.find(
      item => item.id === input.memberId && item.status === 'active'
    )
    if (!member) throw new Error('没有找到启用状态的会员')
    memberId = member.id
    customerName = member.name
    customerPhone = member.phone
  } else if (!customerName || !customerPhone) throw new Error('请填写游客姓名和手机号')

  let serviceName = ''
  let projectId: string | null = null
  let packagePurchaseId: string | null = null
  if (input.serviceType === '套盒手工') {
    if (!memberId) throw new Error('套盒消费只能选择会员')
    const purchase = snapshot.packagePurchases.find(
      item =>
        item.id === input.packagePurchaseId &&
        item.memberId === memberId &&
        item.packageType === '套盒' &&
        item.status === 'active' &&
        item.remainingUses > 0
    )
    if (!purchase) throw new Error('没有找到该会员的可用套盒')
    serviceName = purchase.packageName
    packagePurchaseId = purchase.id
  } else {
    const project = snapshot.projects.find(
      item => item.id === input.projectId && item.status === 'active'
    )
    if (!project) throw new Error('没有找到启用状态的服务项目')
    projectId = project.id
    serviceName = project.name
    input.duration = project.duration
  }

  return {
    customerType: input.customerType,
    memberId,
    customerName,
    customerPhone,
    employee: employee.name,
    serviceType: input.serviceType,
    serviceName,
    projectId,
    packagePurchaseId,
    startsAt: startsAt.toISOString(),
    duration: input.duration,
    note: input.note.trim()
  }
}

function ensureNoConflict(
  snapshot: AppSnapshot,
  appointment: ReturnType<typeof resolveAppointment>,
  excludedId?: string
) {
  const startsAt = new Date(appointment.startsAt).getTime()
  const endsAt = startsAt + appointment.duration * 60_000
  const conflict = snapshot.appointments.find(item => {
    if (
      item.id === excludedId ||
      item.employee !== appointment.employee ||
      item.status === 'cancelled' ||
      item.status === 'no_show'
    )
      return false
    const existingStart = new Date(item.startsAt).getTime()
    const existingEnd = existingStart + item.duration * 60_000
    return startsAt < existingEnd && existingStart < endsAt
  })
  if (conflict)
    throw new Error(
      `该员工在此时间段已有“${conflict.customerName}”的预约，请调整预约时间或服务员工`
    )
}

export function createBrowserAppointment(
  snapshot: AppSnapshot,
  input: AppointmentInput,
  createdBy: string
) {
  const resolved = resolveAppointment(snapshot, input)
  ensureNoConflict(snapshot, resolved)
  const now = new Date().toISOString()
  snapshot.appointments.unshift({
    id: crypto.randomUUID(),
    ...resolved,
    status: 'pending',
    createdBy,
    createdAt: now,
    updatedAt: now,
    completedServiceId: null
  })
}

export function updateBrowserAppointment(
  snapshot: AppSnapshot,
  appointmentId: string,
  input: AppointmentInput
) {
  const current = snapshot.appointments.find(item => item.id === appointmentId)
  if (!current) throw new Error('预约记录不存在')
  if (current.status !== 'pending' && current.status !== 'arrived')
    throw new Error('当前预约状态不允许编辑')
  const resolved = resolveAppointment(snapshot, input)
  ensureNoConflict(snapshot, resolved, appointmentId)
  Object.assign(current, resolved, { updatedAt: new Date().toISOString() })
}

const allowedTransitions: Record<AppointmentStatus, AppointmentStatus[]> = {
  pending: ['arrived', 'cancelled', 'no_show'],
  arrived: ['in_service', 'cancelled'],
  in_service: [],
  completed: [],
  cancelled: [],
  no_show: []
}

export function setBrowserAppointmentStatus(
  snapshot: AppSnapshot,
  appointmentId: string,
  status: AppointmentStatus
) {
  const appointment = snapshot.appointments.find(item => item.id === appointmentId)
  if (!appointment) throw new Error('预约记录不存在')
  if (!allowedTransitions[appointment.status].includes(status))
    throw new Error('预约状态流转无效，请刷新后重试')
  appointment.status = status
  appointment.updatedAt = new Date().toISOString()
}

export function completeBrowserAppointment(
  snapshot: AppSnapshot,
  input: AppointmentCompletionInput
) {
  const appointment = snapshot.appointments.find(item => item.id === input.appointmentId)
  if (!appointment || appointment.status !== 'in_service')
    throw new Error('预约不存在或尚未进入服务中状态')
  if (input.duration <= 0) throw new Error('实际服务时长必须大于0')
  const employee = snapshot.employees.find(
    item => item.name === appointment.employee && item.status === 'active'
  )
  if (!employee) throw new Error('没有找到启用状态的员工')
  const compensation = snapshot.employeeCompensations.find(item => item.employeeId === employee.id)
  const serviceId = crypto.randomUUID()
  const now = new Date().toISOString()
  if (appointment.serviceType === '套盒手工') {
    const purchase = snapshot.packagePurchases.find(
      item =>
        item.id === appointment.packagePurchaseId &&
        item.memberId === appointment.memberId &&
        item.packageType === '套盒' &&
        item.status === 'active' &&
        item.remainingUses > 0
    )
    if (!purchase) throw new Error('预约套盒已结束或没有剩余次数')
    const remainingAfter = purchase.remainingUses - 1
    purchase.remainingUses = remainingAfter
    purchase.lastConsumedAt = now
    if (remainingAfter === 0) purchase.status = 'completed'
    const commission = compensation?.packageServiceCommission ?? 10
    snapshot.services.unshift({
      id: serviceId,
      memberId: appointment.memberId,
      memberName: appointment.customerName,
      employee: appointment.employee,
      serviceName: purchase.packageName,
      serviceType: '套盒手工',
      duration: input.duration,
      amount: 0,
      commission,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      packagePurchaseId: purchase.id,
      createdAt: now,
      status: 'completed'
    })
    snapshot.packageConsumptions.unshift({
      id: crypto.randomUUID(),
      purchaseId: purchase.id,
      employee: appointment.employee,
      duration: input.duration,
      remainingAfter,
      createdAt: now,
      note: '预约服务完成',
      serviceId
    })
  } else {
    const project = snapshot.projects.find(item => item.id === appointment.projectId)
    if (!project) throw new Error('预约缺少服务项目信息')
    const member = appointment.memberId
      ? snapshot.members.find(item => item.id === appointment.memberId)
      : undefined
    const balancePaymentAmount = member ? Math.min(member.balance, project.price) : 0
    const externalPaymentAmount = roundMoney(project.price - balancePaymentAmount)
    if (
      externalPaymentAmount > 0 &&
      !['微信支付', '支付宝', '现金', '银行卡'].includes(input.externalPaymentMethod)
    )
      throw new Error('请选择实际支付方式')
    const giftDeduction = member ? Math.min(member.giftBalance, balancePaymentAmount) : 0
    const principalDeduction = balancePaymentAmount - giftDeduction
    if (member) {
      member.giftBalance = roundMoney(member.giftBalance - giftDeduction)
      member.principalBalance = roundMoney(member.principalBalance - principalDeduction)
      member.balance = roundMoney(member.principalBalance + member.giftBalance)
      member.totalConsumption += project.price
      member.lastVisit = now
    }
    const paymentMethod =
      balancePaymentAmount > 0 && externalPaymentAmount > 0
        ? `会员余额+${input.externalPaymentMethod}`
        : balancePaymentAmount > 0
          ? '会员余额'
          : input.externalPaymentMethod
    const transactionId = crypto.randomUUID()
    snapshot.services.unshift({
      id: serviceId,
      memberId: appointment.memberId,
      memberName: appointment.customerName,
      employee: appointment.employee,
      serviceName: project.name,
      serviceType: '普通手工',
      duration: project.duration,
      amount: project.price,
      commission: compensation?.normalServiceCommission ?? 5,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      packagePurchaseId: null,
      projectId: project.id,
      balancePaymentAmount,
      externalPaymentAmount,
      paymentMethod,
      giftDeduction,
      principalDeduction,
      transactionId,
      createdAt: now,
      status: 'completed'
    })
    snapshot.transactions.unshift({
      id: transactionId,
      memberId: member?.id ?? null,
      memberName: appointment.customerName,
      type: 'consume',
      amount: project.price,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: member?.balance ?? 0,
      paymentMethod,
      item: project.name,
      employee: appointment.employee,
      createdAt: now,
      note: '预约普通消费自动结算',
      status: 'active',
      sourceType: 'service',
      sourceId: serviceId
    })
  }
  if (appointment.memberId) {
    const member = snapshot.members.find(item => item.id === appointment.memberId)
    if (member) member.lastVisit = now
  }
  appointment.status = 'completed'
  appointment.completedServiceId = serviceId
  appointment.updatedAt = now
}

export function normalizeAppointments(snapshot: AppSnapshot) {
  snapshot.appointments ??= []
  snapshot.appointments.forEach(item => {
    item.projectId ??= null
  })
}
