import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'

type TableRecords<T> = Ref<readonly T[]> | ComputedRef<readonly T[]>

export function useTablePagination<T>(records: TableRecords<T>, pageSize = 10) {
  const currentPage = ref(1)
  const currentPageSize = ref(pageSize)
  const paginatedRecords = computed(() => {
    const start = (currentPage.value - 1) * currentPageSize.value
    return records.value.slice(start, start + currentPageSize.value)
  })

  watch([records, currentPageSize], () => (currentPage.value = 1))

  return { currentPage, pageSize: currentPageSize, paginatedRecords }
}
