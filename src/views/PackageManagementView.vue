<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { PACKAGE_TYPES } from '../config/options'
import { addPackage, salonStore, setPackageStatus, updatePackage } from '../data/repository'
import type { PackageDefinition, PackageDefinitionInput, PackageType } from '../types'
import { currency, fullDateTime } from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import { positiveNumberRule, requiredTextRule, validateForm } from '../utils/validation'

type PackageFilters = { name: string; packageType: PackageType | ''; status: string }

const filterForm = reactive<PackageFilters>({ name: '', packageType: '', status: '' })
const appliedFilters = reactive<PackageFilters>({ ...filterForm })
const modalVisible = ref(false)
const saving = ref(false)
const editingPackage = ref<PackageDefinition | null>(null)
const packageFormRef = ref<FormInstance>()
const packageForm = reactive<PackageDefinitionInput>({
  name: '',
  price: 0,
  totalUses: 1,
  packageType: '套盒'
})

const packageRules: FormRules<PackageDefinitionInput> = {
  name: [requiredTextRule('请输入套餐名称')],
  price: [positiveNumberRule('请输入大于 0 的套餐价格')],
  totalUses: [positiveNumberRule('请输入大于 0 的可用次数')],
  packageType: [{ required: true, message: '请选择套餐类型', trigger: 'change' }]
}

const filteredPackages = computed(() =>
  salonStore.packages.filter(item => {
    const name = appliedFilters.name.trim()
    return (
      (!name || item.name.includes(name)) &&
      (!appliedFilters.packageType || item.packageType === appliedFilters.packageType) &&
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
  Object.assign(filterForm, { name: '', packageType: '', status: '' })
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
          packageType: item.packageType
        }
      : { name: '', price: 0, totalUses: 1, packageType: '套盒' }
  )
  modalVisible.value = true
}

async function submitPackage() {
  if (!(await validateForm(packageFormRef.value))) return
  saving.value = true
  try {
    if (editingPackage.value) await updatePackage(editingPackage.value.id, { ...packageForm })
    else await addPackage({ ...packageForm })
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
        <el-table-column prop="totalUses" label="可用次数" min-width="100" />
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
          </el-form-item>
          <el-form-item label="可用次数" prop="totalUses">
            <el-input-number
              v-model="packageForm.totalUses"
              :min="1"
              :precision="0"
              :controls="false"
              align="left"
            />
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
