<script setup lang="ts">
import type { Component } from 'vue'
defineProps<{
  label: string
  value: string
  hint: string
  trend?: string
  icon: Component
  tone: 'rose' | 'gold' | 'violet' | 'green'
}>()
</script>

<template>
  <article class="stat-card" :class="tone">
    <div class="stat-icon"><component :is="icon" :size="21" /></div>
    <div class="stat-head">
      <span>{{ label }}</span>
      <b v-if="trend">{{ trend }}</b>
    </div>
    <strong>{{ value }}</strong>
    <p>{{ hint }}</p>
  </article>
</template>
