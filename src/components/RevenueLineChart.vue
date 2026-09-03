<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { LineChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { init, use, type EChartsCoreOption, type EChartsType } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { currency } from '../utils'

interface RevenuePoint {
  label: string
  amount: number
}

const props = defineProps<{
  data: RevenuePoint[]
  name?: string
}>()

use([LineChart, GridComponent, TooltipComponent, CanvasRenderer])

const chartElement = ref<HTMLElement>()
let chart: EChartsType | undefined
let resizeObserver: ResizeObserver | undefined

function yAxisLabel(value: number) {
  if (value >= 10000) return `${Number((value / 10000).toFixed(1))}万`
  return `${value}`
}

function chartOption(): EChartsCoreOption {
  return {
    animationDuration: 450,
    color: ['#d8657e'],
    grid: {
      top: 24,
      right: 18,
      bottom: 10,
      left: 12,
      containLabel: true
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(255, 255, 255, 0.96)',
      borderColor: '#eadde0',
      borderWidth: 1,
      padding: [9, 12],
      textStyle: { color: '#5f5155', fontSize: 12 },
      axisPointer: {
        type: 'line',
        lineStyle: { color: '#d8c4c9', type: 'dashed' }
      },
      valueFormatter: (value: unknown) => currency(Number(value))
    },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: props.data.map(item => item.label),
      axisLine: { lineStyle: { color: '#e8dfe1' } },
      axisTick: { show: false },
      axisLabel: { color: '#978b8e', fontSize: 12, margin: 12 }
    },
    yAxis: {
      type: 'value',
      min: 0,
      splitNumber: 4,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: {
        color: '#a4999c',
        fontSize: 12,
        formatter: yAxisLabel
      },
      splitLine: {
        lineStyle: { color: '#eee8e6', type: 'dashed' }
      }
    },
    series: [
      {
        name: props.name ?? '营收',
        type: 'line',
        data: props.data.map(item => item.amount),
        smooth: 0.35,
        symbol: 'circle',
        symbolSize: 8,
        showSymbol: true,
        lineStyle: { width: 3, color: '#d8657e' },
        itemStyle: {
          color: '#ffffff',
          borderColor: '#d8657e',
          borderWidth: 3
        },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(216, 101, 126, 0.3)' },
              { offset: 1, color: 'rgba(216, 101, 126, 0.02)' }
            ]
          }
        },
        emphasis: { focus: 'series' }
      }
    ]
  }
}

function renderChart() {
  chart?.setOption(chartOption(), true)
}

watch(() => [props.data, props.name], renderChart, { deep: true })

onMounted(() => {
  if (!chartElement.value) return
  chart = init(chartElement.value)
  renderChart()
  resizeObserver = new ResizeObserver(() => chart?.resize())
  resizeObserver.observe(chartElement.value)
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  chart?.dispose()
})
</script>

<template>
  <div ref="chartElement" class="revenue-line-chart" />
</template>

<style scoped>
.revenue-line-chart {
  width: 100%;
  height: 100%;
}
</style>
