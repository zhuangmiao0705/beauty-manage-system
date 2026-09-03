<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { addProject, salonStore, setProjectStatus, updateProject } from '../data/repository'
import type { ProjectDefinition, ProjectDefinitionInput } from '../types'
import { currency, fullDateTime } from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import { positiveNumberRule, requiredTextRule, validateForm } from '../utils/validation'

type ProjectFilters = { name: string; status: '' | 'active' | 'inactive' }

const filterForm = reactive<ProjectFilters>({ name: '', status: '' })
const appliedFilters = reactive<ProjectFilters>({ ...filterForm })
const modalVisible = ref(false)
const saving = ref(false)
const editingProject = ref<ProjectDefinition | null>(null)
const formRef = ref<FormInstance>()
const form = reactive<ProjectDefinitionInput>({ name: '', duration: 60, price: 0 })
const rules: FormRules<ProjectDefinitionInput> = {
  name: [requiredTextRule('请输入项目名称')],
  duration: [positiveNumberRule('请输入大于0的项目时长')],
  price: [positiveNumberRule('请输入大于0的项目价格')]
}

const filteredProjects = computed(() =>
  salonStore.projects.filter(item => {
    const name = appliedFilters.name.trim()
    return (
      (!name || item.name.includes(name)) &&
      (!appliedFilters.status || item.status === appliedFilters.status)
    )
  })
)
const {
  currentPage,
  pageSize,
  paginatedRecords: paginatedProjects
} = useTablePagination(filteredProjects)

function queryProjects() {
  Object.assign(appliedFilters, filterForm)
}

function resetFilters() {
  Object.assign(filterForm, { name: '', status: '' })
  queryProjects()
}

function openProject(item?: ProjectDefinition) {
  editingProject.value = item ?? null
  Object.assign(
    form,
    item
      ? { name: item.name, duration: item.duration, price: item.price }
      : { name: '', duration: 60, price: 0 }
  )
  modalVisible.value = true
}

async function submitProject() {
  if (!(await validateForm(formRef.value))) return
  saving.value = true
  try {
    if (editingProject.value) await updateProject(editingProject.value.id, { ...form })
    else await addProject({ ...form })
    modalVisible.value = false
    notify(editingProject.value ? '项目已更新' : '项目已创建')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function toggleProject(item: ProjectDefinition) {
  const status = item.status === 'active' ? 'inactive' : 'active'
  try {
    await setProjectStatus(item.id, status)
    notify(status === 'active' ? '项目已启用' : '项目已停用')
  } catch (reason) {
    notify(errorMessage(reason, '操作失败'), 'error')
  }
}
</script>

<template>
  <div class="page">
    <section class="section-toolbar">
      <div />
      <el-button type="primary" @click="openProject()">
        <Plus :size="17" />
        新增项目
      </el-button>
    </section>
    <section class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar">
        <label class="table-filter-field">
          <span>项目名称</span>
          <el-input
            v-model="filterForm.name"
            clearable
            class="table-filter-input"
            placeholder="请输入项目名称"
            @keyup.enter="queryProjects"
          />
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
          <el-button type="primary" @click="queryProjects">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="resetFilters">重置</el-button>
        </div>
      </div>
      <el-table :data="paginatedProjects" row-key="id" stripe class="salon-table">
        <el-table-column prop="name" label="项目名称" min-width="190" />
        <el-table-column label="项目时长" min-width="110">
          <template #default="{ row }">{{ row.duration }} 分钟</template>
        </el-table-column>
        <el-table-column label="项目价格" min-width="120">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.price) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag round :type="row.status === 'active' ? 'success' : 'info'">
              {{ row.status === 'active' ? '启用' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="更新时间" min-width="165">
          <template #default="{ row }">{{ fullDateTime(row.updatedAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="openProject(row as ProjectDefinition)">编辑</el-button>
            <el-button
              size="small"
              :type="row.status === 'active' ? 'danger' : 'success'"
              plain
              @click="toggleProject(row as ProjectDefinition)"
            >
              {{ row.status === 'active' ? '停用' : '启用' }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="currentPage"
        v-model:page-size="pageSize"
        :total="filteredProjects.length"
      />
    </section>

    <BaseModal
      v-if="modalVisible"
      :title="editingProject ? '编辑项目' : '新增项目'"
      subtitle="修改配置不会影响已经完成的历史服务"
      @close="modalVisible = false"
    >
      <el-form ref="formRef" :model="form" :rules="rules" scroll-to-error label-position="top">
        <div class="form-grid">
          <el-form-item class="span-2" label="项目名称" prop="name">
            <el-input v-model="form.name" clearable placeholder="请输入项目名称" />
          </el-form-item>
          <el-form-item label="项目时长（分钟）" prop="duration">
            <el-input-number
              v-model="form.duration"
              :min="1"
              :precision="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="项目价格" prop="price">
            <el-input-number v-model="form.price" :min="0" :controls="false" align="left" />
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="modalVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitProject">保存</el-button>
      </template>
    </BaseModal>
  </div>
</template>
