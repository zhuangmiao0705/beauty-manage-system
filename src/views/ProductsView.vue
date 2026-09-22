<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { AlertTriangle, Plus, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import { addProduct, addProductStock, salonStore, updateProduct } from '../data/repository'
import type { Product, ProductInput } from '../types'
import { currency, fullDateTime } from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import {
  nonNegativeNumberRule,
  positiveNumberRule,
  requiredTextRule,
  validateForm
} from '../utils/validation'

type StockForm = {
  quantity: number
}

const keyword = ref('')
const productModalVisible = ref(false)
const stockModalVisible = ref(false)
const detailModalVisible = ref(false)
const saving = ref(false)
const editingProduct = ref<Product | null>(null)
const selectedProduct = ref<Product | null>(null)
const productFormRef = ref<FormInstance>()
const stockFormRef = ref<FormInstance>()
const form = reactive<ProductInput>({ name: '', unitPrice: 0, stock: 0 })
const stockForm = reactive<StockForm>({ quantity: 1 })
const productRules: FormRules<ProductInput> = {
  name: [requiredTextRule('请输入产品名称')],
  unitPrice: [nonNegativeNumberRule('成本不能小于0')],
  stock: [nonNegativeNumberRule('初始库存不能小于0')]
}
const stockRules: FormRules<StockForm> = {
  quantity: [positiveNumberRule('增加库存数量必须大于0')]
}

const filteredProducts = computed(() => {
  const value = keyword.value.trim()
  return salonStore.products.filter(item => !value || item.name.includes(value))
})
const lowStockCount = computed(() => salonStore.products.filter(item => item.stock < 10).length)
const selectedDetails = computed(() =>
  salonStore.productConsumptions.filter(
    item => item.productId === selectedProduct.value?.id && item.status === 'active'
  )
)

function openProduct(item?: Product) {
  editingProduct.value = item ?? null
  Object.assign(
    form,
    item
      ? { name: item.name, unitPrice: item.unitPrice, stock: item.stock }
      : { name: '', unitPrice: 0, stock: 0 }
  )
  productModalVisible.value = true
}

async function submitProduct() {
  if (!(await validateForm(productFormRef.value))) return
  saving.value = true
  try {
    if (editingProduct.value) await updateProduct(editingProduct.value.id, { ...form })
    else await addProduct({ ...form })
    productModalVisible.value = false
    notify(editingProduct.value ? '产品已更新' : '产品已创建')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

function openStock(item: Product) {
  selectedProduct.value = item
  stockForm.quantity = 1
  stockModalVisible.value = true
}

async function submitStock() {
  if (!(await validateForm(stockFormRef.value)) || !selectedProduct.value) return
  saving.value = true
  try {
    await addProductStock(selectedProduct.value.id, stockForm.quantity)
    stockModalVisible.value = false
    notify('库存已增加')
  } catch (reason) {
    notify(errorMessage(reason, '入库失败'), 'error')
  } finally {
    saving.value = false
  }
}

function openDetails(item: Product) {
  selectedProduct.value = item
  detailModalVisible.value = true
}

const rowClassName = ({ row }: { row: Product }) => (row.stock < 10 ? 'low-stock-row' : '')
</script>

<template>
  <div class="page material-page">
    <section v-if="lowStockCount" class="inventory-warning">
      <AlertTriangle :size="18" />
      <span>当前有 {{ lowStockCount }} 个产品库存低于 10，请及时补充库存。</span>
    </section>

    <section class="panel table-panel material-panel">
      <div class="material-toolbar">
        <el-input v-model="keyword" clearable class="material-search" placeholder="按产品名称查询">
          <template #prefix><Search :size="15" /></template>
        </el-input>
        <el-button type="primary" @click="openProduct()">
          <Plus :size="17" />
          新增产品
        </el-button>
      </div>

      <el-table
        :data="filteredProducts"
        row-key="id"
        class="salon-table material-table"
        :row-class-name="rowClassName"
        empty-text="暂无产品，请先新增产品"
      >
        <el-table-column prop="name" label="产品名称" min-width="200" />
        <el-table-column label="成本" min-width="130">
          <template #default="{ row }">{{ currency(row.unitPrice) }}</template>
        </el-table-column>
        <el-table-column label="库存" min-width="130">
          <template #default="{ row }">
            <strong>{{ row.stock }}</strong>
            <span v-if="row.stock < 10" class="stock-warning-text">库存不足</span>
          </template>
        </el-table-column>
        <el-table-column label="更新时间" min-width="170">
          <template #default="{ row }">{{ fullDateTime(row.updatedAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="270" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="openDetails(row as Product)">消耗明细</el-button>
            <el-button size="small" @click="openProduct(row as Product)">编辑</el-button>
            <el-button size="small" type="primary" plain @click="openStock(row as Product)">
              增加库存
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <BaseModal
      v-if="productModalVisible"
      :title="editingProduct ? '编辑产品' : '新增产品'"
      subtitle="成本用于计算产品库存成本，库存可按实际盘点结果修正"
      @close="productModalVisible = false"
    >
      <el-form
        ref="productFormRef"
        :model="form"
        :rules="productRules"
        label-position="top"
        scroll-to-error
      >
        <div class="form-grid">
          <el-form-item class="span-2" label="产品名称" prop="name">
            <el-input v-model="form.name" clearable placeholder="请输入产品名称" />
          </el-form-item>
          <el-form-item label="成本" prop="unitPrice">
            <el-input-number
              v-model="form.unitPrice"
              :min="0"
              :precision="2"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item :label="editingProduct ? '当前库存' : '初始库存'" prop="stock">
            <el-input-number v-model="form.stock" :min="0" :controls="false" align="left" />
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="productModalVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitProduct">保存</el-button>
      </template>
    </BaseModal>

    <BaseModal
      v-if="stockModalVisible"
      title="增加库存"
      :subtitle="
        selectedProduct ? `产品：${selectedProduct.name}，当前库存：${selectedProduct.stock}` : ''
      "
      @close="stockModalVisible = false"
    >
      <el-form
        ref="stockFormRef"
        :model="stockForm"
        :rules="stockRules"
        label-position="top"
        scroll-to-error
      >
        <el-form-item label="本次增加数量" prop="quantity">
          <el-input-number
            v-model="stockForm.quantity"
            :min="0.01"
            :controls="false"
            align="left"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="stockModalVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitStock">确认入库</el-button>
      </template>
    </BaseModal>

    <BaseModal
      v-if="detailModalVisible"
      title="产品消耗明细"
      :subtitle="selectedProduct?.name ?? ''"
      wide
      @close="detailModalVisible = false"
    >
      <el-table
        :data="selectedDetails"
        row-key="id"
        stripe
        max-height="460"
        empty-text="暂无消耗记录"
      >
        <el-table-column label="消耗时间" min-width="165">
          <template #default="{ row }">{{ fullDateTime(row.consumedAt) }}</template>
        </el-table-column>
        <el-table-column prop="memberName" label="顾客" min-width="120" />
        <el-table-column prop="employee" label="服务员工" min-width="110" />
        <el-table-column label="来源" min-width="110">
          <template #default="{ row }">
            {{ row.sourceType === 'project' ? '单次项目' : '套餐' }}
          </template>
        </el-table-column>
        <el-table-column prop="quantity" label="消耗数量" min-width="100" />
      </el-table>
    </BaseModal>
  </div>
</template>

<style scoped>
.material-page {
  display: block;
}

.material-page > * + * {
  margin-top: 16px;
}

.inventory-warning {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 12px 16px;
  color: #a61b1b;
  background: #fff1f1;
  border: 1px solid #ffd2d2;
  border-radius: 12px;
  font-size: 14px;
}

.material-panel {
  overflow: hidden;
}

.material-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 16px 0;
  margin-bottom: 18px;
}

.material-search {
  width: min(320px, 100%);
}

.stock-warning-text {
  display: inline-block;
  margin-left: 8px;
  padding: 2px 7px;
  border-radius: 999px;
  color: #b42318;
  background: #fee4e2;
  font-size: 12px;
}

:deep(.el-table .low-stock-row) {
  --el-table-tr-bg-color: #fff0f0;
  --el-table-row-hover-bg-color: #ffe4e4;
  color: #a61b1b;
}

@media (max-width: 720px) {
  .material-toolbar {
    align-items: stretch;
    flex-direction: column;
  }

  .material-search {
    width: 100%;
  }
}
</style>
