<script setup lang="ts">
withDefaults(defineProps<{ total: number; pageSizes?: number[] }>(), {
  pageSizes: () => [10, 20, 50]
})
const currentPage = defineModel<number>({ default: 1 })
const pageSize = defineModel<number>('pageSize', { default: 10 })
</script>

<template>
  <div class="table-pagination">
    <el-pagination
      v-model:current-page="currentPage"
      v-model:page-size="pageSize"
      background
      layout="total, sizes, prev, pager, next"
      :page-sizes="pageSizes"
      :total="total"
    />
  </div>
</template>

<style scoped>
.table-pagination {
  display: flex;
  justify-content: flex-end;
  margin: 16px 0;
  overflow-x: auto;
}
</style>
