<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { PACKAGE_LIMIT_TYPES, PACKAGE_TYPES } from '../config/options'
import { addPackage, salonStore, setPackageStatus, updatePackage } from '../data/repository'
import type {
  PackageDefinition,
  PackageDefinitionInput,
  PackageLimitType,
  PackageType
} from '../types'
import { currency, fullDateTime, packageDefinitionLimitText } from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import {
  nonNegativeNumberRule,
  positiveNumberRule,
  requiredTextRule,
  validateForm
} from '../utils/validation'

type PackageFilters = {
  name: string
  packageType: PackageType | ''
  limitType: PackageLimitType | ''
  status: string
}

const filterForm = reactive<PackageFilters>({
  name: '',
  packageType: '',
  limitType: '',
  status: ''
})
const appliedFilters = reactive<PackageFilters>({ ...filterForm })
const modalVisible = ref(false)
const saving = ref(false)
const editingPackage = ref<PackageDefinition | null>(null)
const packageFormRef = ref<FormInstance>()
const packageForm = reactive<PackageDefinitionInput>({
  name: '',
  price: 0,
  totalUses: 1,
  limitType: 'count',
  validityDays: 30,
  packageType: '套盒'
})

const packageRules = computed<FormRules<PackageDefinitionInput>>(() => ({
  name: [requiredTextRule('请输入套餐名称')],
  price: [nonNegativeNumberRule('套餐价格不能小于 0')],
  totalUses:
    packageForm.limitType === 'count' ? [positiveNumberRule('请输入大于 0 的可用次数')] : [],
  validityDays:
    packageForm.limitType === 'time' ? [positiveNumberRule('请输入大于 0 的有效天数')] : [],
  limitType: [{ required: true, message: '请选择限制方式', trigger: 'change' }],
  packageType: [{ required: true, message: '请选择套餐类型', trigger: 'change' }]
}))

const filteredPackages = computed(() =>
  salonStore.packages.filter(item => {
    const name = appliedFilters.name.trim()
    return (
      (!name || item.name.includes(name)) &&
      (!appliedFilters.packageType || item.packageType === appliedFilters.packageType) &&
      (!appliedFilters.limitType || item.limitType === appliedFilters.limitType) &&
      (!appliedFilters.status || item.status === appliedFilters.status)
    )
  })
)
const {
  currentPage,
  pageSize,
  paginatedRecords: paginatedPackages
} = useTablePagination(filteredPackages)

function queryPackages() {
  Object.assign(appliedFilters, filterForm)
}

function resetFilters() {
  Object.assign(filterForm, { name: '', packageType: '', limitType: '', status: '' })
  queryPackages()
}

function openPackage(item?: PackageDefinition) {
  editingPackage.value = item ?? null
  Object.assign(
    packageForm,
    item
      ? {
          name: item.name,
          price: item.price,
          totalUses: item.totalUses,
          limitType: item.limitType,
          validityDays: item.validityDays,
          packageType: item.packageType
        }
      : {
          name: '',
          price: 0,
          totalUses: 1,
          limitType: 'count',
          validityDays: 30,
          packageType: '套盒'
        }
  )
  modalVisible.value = true
}

async function submitPackage() {
  if (!(await validateForm(packageFormRef.value))) return
  const input: PackageDefinitionInput = {
    ...packageForm,
    totalUses: packageForm.limitType === 'count' ? packageForm.totalUses : 1,
    validityDays: packageForm.limitType === 'time' ? packageForm.validityDays : 0
  }
  saving.value = true
  try {
    if (editingPackage.value) await updatePackage(editingPackage.value.id, input)
    else await addPackage(input)
    modalVisible.value = false
    notify(editingPackage.value ? '套餐配置已更新' : '套餐配置已创建')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function togglePackage(item: PackageDefinition) {
  const nextStatus = item.status === 'active' ? 'inactive' : 'active'
  try {
    await setPackageStatus(item.id, nextStatus)
    notify(nextStatus === 'inactive' ? '套餐已停用' : '套餐已启用')
  } catch (reason) {
    notify(errorMessage(reason, '操作失败'), 'error')
  }
}
</script>

<template>
  <div class="page">
    <section class="section-toolbar">
      <div />
      <el-button type="primary" @click="openPackage()">
        <Plus :size="17" />
        新增套餐
      </el-button>
    </section>

    <section class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar">
        <label class="table-filter-field">
          <span>套餐名称</span>
          <el-input
            v-model="filterForm.name"
            clearable
            class="table-filter-input"
            placeholder="请输入套餐名称"
          />
        </label>
        <label class="table-filter-field">
          <span>套餐类型</span>
          <el-select
            v-model="filterForm.packageType"
            clearable
            class="table-filter-select"
            placeholder="请选择类型"
          >
            <el-option v-for="item in PACKAGE_TYPES" :key="item" :label="item" :value="item" />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>限制方式</span>
          <el-select
            v-model="filterForm.limitType"
            clearable
            class="table-filter-select"
            placeholder="请选择限制方式"
          >
            <el-option
              v-for="item in PACKAGE_LIMIT_TYPES"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>状态</span>
          <el-select
            v-model="filterForm.status"
            clearable
            class="table-filter-select"
            placeholder="请选择状态"
          >
            <el-option label="启用" value="active" />
            <el-option label="停用" value="inactive" />
          </el-select>
        </label>
        <div class="service-filter-actions">
          <el-button type="primary" @click="queryPackages">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="resetFilters">重置</el-button>
        </div>
      </div>
      <el-table :data="paginatedPackages" row-key="id" stripe class="salon-table">
        <el-table-column prop="name" label="套餐名称" min-width="180" />
        <el-table-column label="套餐类型" min-width="100">
          <template #default="{ row }">
            <el-tag round :type="row.packageType === '套盒' ? 'warning' : 'info'">
              {{ row.packageType }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="价格" min-width="120">
          <template #default="{ row }">{{ currency(row.price) }}</template>
        </el-table-column>
        <el-table-column label="限制方式" min-width="100">
          <template #default="{ row }">
            {{ row.limitType === 'time' ? '时间限制' : '次数限制' }}
          </template>
        </el-table-column>
        <el-table-column label="使用限制" min-width="100">
          <template #default="{ row }">
            {{ packageDefinitionLimitText(row as PackageDefinition) }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag round :type="row.status === 'active' ? 'success' : 'info'">
              {{ row.status === 'active' ? '启用' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="创建时间" min-width="165">
          <template #default="{ row }">{{ fullDateTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="openPackage(row as PackageDefinition)">编辑</el-button>
            <el-button
              size="small"
              :type="row.status === 'active' ? 'danger' : 'success'"
              plain
              @click="togglePackage(row as PackageDefinition)"
            >
              {{ row.status === 'active' ? '停用' : '启用' }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="currentPage"
        v-model:page-size="pageSize"
        :total="filteredPackages.length"
      />
    </section>

    <BaseModal
      v-if="modalVisible"
      :title="editingPackage ? '编辑套餐' : '新增套餐'"
      subtitle="修改配置不会影响已经售出的套餐"
      @close="modalVisible = false"
    >
      <el-form
        ref="packageFormRef"
        :model="packageForm"
        :rules="packageRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item class="span-2" label="套餐名称" prop="name">
            <el-input v-model="packageForm.name" clearable placeholder="请输入套餐名称" />
          </el-form-item>
          <el-form-item label="套餐类型" prop="packageType">
            <el-select v-model="packageForm.packageType">
              <el-option v-for="item in PACKAGE_TYPES" :key="item" :label="item" :value="item" />
            </el-select>
          </el-form-item>
          <el-form-item label="固定价格" prop="price">
            <el-input-number v-model="packageForm.price" :min="0" :controls="false" align="left" />
            <div class="form-tip">价格填写 0 时，将作为赠送套餐使用。</div>
          </el-form-item>
          <el-form-item label="限制方式" prop="limitType">
            <el-select v-model="packageForm.limitType">
              <el-option
                v-for="item in PACKAGE_LIMIT_TYPES"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
          <el-form-item v-if="packageForm.limitType === 'count'" label="可用次数" prop="totalUses">
            <el-input-number
              v-model="packageForm.totalUses"
              :min="1"
              :precision="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item v-else label="有效天数" prop="validityDays">
            <el-input-number
              v-model="packageForm.validityDays"
              :min="1"
              :precision="0"
              :controls="false"
              align="left"
            />
            <div class="form-tip">从会员购买当天开始计算，有效期内不限使用次数。</div>
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="modalVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitPackage">保存配置</el-button>
      </template>
    </BaseModal>
  </div>
</template>
