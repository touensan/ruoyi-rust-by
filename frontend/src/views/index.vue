<template>
  <div class="app-container home">
    <section class="intro-panel">
      <div class="intro-copy">
        <p class="product-code">ruoyi-rust-by</p>
        <h1>RuoYi-Rust BY</h1>
        <p class="product-summary">
          这是一个基于 Axum、Rust 和若依界面开发的 RuoYi 风格开源项目。
        </p>
        <div class="intro-actions">
          <el-button type="primary" @click="goTarget(officialSite)">访问官网</el-button>
          <el-tag type="success" effect="plain">当前版本 v{{ productVersion }}</el-tag>
        </div>
      </div>

      <div class="runtime-box">
        <div>
          <span>项目名称</span>
          <strong>RuoYi-Rust BY</strong>
        </div>
        <div>
          <span>英文代号</span>
          <strong>ruoyi-rust-by</strong>
        </div>
        <div>
          <span>官网地址</span>
          <el-link :href="officialSite" target="_blank" type="primary">{{ officialSite }}</el-link>
        </div>
      </div>
    </section>

    <section class="stack-section">
      <div class="section-heading">
        <h2>技术栈</h2>
        <p>后端以 Axum 为核心，前端保留 Vue3 TypeScript 管理端，部署面向宝塔 / Nginx / MySQL 环境。</p>
      </div>

      <el-row :gutter="16">
        <el-col v-for="group in stackGroups" :key="group.title" :xs="24" :md="8">
          <el-card shadow="never" class="stack-card">
            <template #header>
              <span>{{ group.title }}</span>
            </template>
            <div class="stack-list">
              <el-tag v-for="item in group.items" :key="item" effect="plain">{{ item }}</el-tag>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </section>

    <section class="changelog-section">
      <div class="section-heading">
        <h2>更新日志</h2>
        <p>记录 RuoYi-Rust BY 的版本演进，只保留本项目的真实变更。</p>
      </div>

      <el-scrollbar class="changelog-scroll" max-height="260px">
        <el-collapse v-model="activeLog" accordion class="changelog-collapse">
          <el-collapse-item v-for="log in changelog" :key="log.version" :name="log.version">
            <template #title>
              <div class="changelog-title-row">
                <span class="log-version">{{ log.version }}</span>
                <span class="log-date">{{ log.date }}</span>
                <span class="log-title">{{ log.title }}</span>
                <span class="log-status" :class="{ 'is-current': log.current }">{{ log.status }}</span>
              </div>
            </template>
            <div class="changelog-detail">
              <p>{{ log.summary }}</p>
              <ul>
                <li v-for="item in log.items" :key="item">{{ item }}</li>
              </ul>
            </div>
          </el-collapse-item>
        </el-collapse>
      </el-scrollbar>
    </section>

    <section class="credits">
      <span>致谢：</span>
      <el-link href="https://github.com/touensan/ruoyi-go-by" target="_blank" type="primary">
        touensan/ruoyi-go-by
      </el-link>
      <span>、</span>
      <el-link href="https://gitcode.com/yangzongzhuan/RuoYi-Vue3/tree/typescript" target="_blank" type="primary">
        RuoYi-Vue3 TypeScript
      </el-link>
    </section>
  </div>
</template>

<script setup lang="ts">
const productVersion = '0.1.0'
const officialSite = 'https://github.com/touensan/ruoyi-rust-by'
const activeLog = ref<string>('')

const stackGroups = [
  {
    title: '后端技术',
    items: ['Rust', 'Axum 0.8', 'MySQL 5.7/8', 'Bearer Token', 'CSV']
  },
  {
    title: '前端技术',
    items: ['Vue 3.5', 'Vite 6', 'TypeScript', 'Element Plus', 'Pinia', 'Vue Router 4', 'Axios']
  },
  {
    title: '部署运行',
    items: ['Rust 二进制', 'Nginx', 'Cargo', '数据库迁移', 'Vue 静态资源']
  }
]

const changelog = [{
  version: 'v0.1.0', date: '2026-09-05', title: 'Rust 首个预发布版本', status: '当前版本', current: true,
  summary: '保留原生若依界面，后端使用 Rust、Axum 和 MySQL。功能范围与部署步骤请参阅仓库说明。',
  items: ['接入用户、角色、菜单、部门、字典和参数管理。', '增加可撤销会话、用户数据范围和配置密钥加密。', '提供 Rust 模板、易支付、SMTP、定时任务、Redis 监控与 Excel 用户导入。']
}]
</script>

<style scoped lang="scss">
.home {
  color: var(--el-text-color-primary);
}

.intro-panel {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  gap: 24px;
  padding: 28px 32px;
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  background: var(--el-bg-color);
}

.product-code {
  margin: 0 0 8px;
  color: var(--el-color-primary);
  font-size: 13px;
  font-weight: 600;
}

.intro-copy h1 {
  margin: 0;
  font-size: 32px;
  line-height: 1.25;
  font-weight: 650;
}

.product-summary {
  max-width: 760px;
  margin: 16px 0 0;
  color: var(--el-text-color-regular);
  font-size: 16px;
  line-height: 1.8;
}

.intro-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  margin-top: 22px;
}

.runtime-box {
  display: grid;
  gap: 14px;
  padding: 18px;
  border-radius: 8px;
  background: var(--el-fill-color-lighter);
}

.runtime-box div {
  min-width: 0;
}

.runtime-box span {
  display: block;
  margin-bottom: 6px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.runtime-box strong {
  display: block;
  font-size: 17px;
  font-weight: 650;
}

.runtime-box :deep(.el-link__inner) {
  word-break: break-all;
}

.stack-section {
  margin-top: 20px;
}

.changelog-section {
  margin-top: 20px;
}

.section-heading {
  margin-bottom: 14px;
}

.section-heading h2 {
  margin: 0;
  font-size: 22px;
  font-weight: 650;
}

.section-heading p {
  margin: 8px 0 0;
  color: var(--el-text-color-secondary);
  line-height: 1.7;
}

.stack-card {
  height: 100%;
  border-radius: 8px;
}

.stack-card :deep(.el-card__header) {
  font-weight: 650;
}

.stack-list {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.changelog-scroll {
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  background: var(--el-bg-color);
}

.changelog-collapse {
  border: 0;
}

.changelog-collapse :deep(.el-collapse-item__wrap) {
  border-bottom: 0;
}

.changelog-collapse :deep(.el-collapse-item__header) {
  height: 42px;
  padding: 0 12px 0 16px;
  border-bottom-color: var(--el-border-color-lighter);
  color: var(--el-text-color-primary);
  line-height: 42px;
  transition: background-color 0.2s ease;
}

.changelog-collapse :deep(.el-collapse-item__header:hover) {
  background: var(--el-fill-color-lighter);
}

.changelog-collapse :deep(.el-collapse-item__header.is-active) {
  background: var(--el-color-primary-light-9);
  border-bottom-color: var(--el-color-primary-light-7);
}

.changelog-collapse :deep(.el-collapse-item__arrow) {
  margin-left: 10px;
  color: var(--el-text-color-secondary);
}

.changelog-collapse :deep(.el-collapse-item__content) {
  padding: 10px 16px 14px 16px;
}

.changelog-title-row {
  display: grid;
  grid-template-columns: 76px 104px minmax(0, 1fr) 72px;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-width: 0;
}

.log-version {
  color: var(--el-text-color-primary);
  font-weight: 650;
}

.log-date {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.log-title {
  min-width: 0;
  overflow: hidden;
  color: var(--el-text-color-regular);
  font-size: 14px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-status {
  justify-self: end;
  width: 64px;
  height: 24px;
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-lighter);
  font-size: 12px;
  line-height: 22px;
  text-align: center;
  white-space: nowrap;
}

.log-status.is-current {
  border-color: var(--el-color-success-light-5);
  color: var(--el-color-success);
  background: var(--el-color-success-light-9);
}

.changelog-detail {
  padding-left: 180px;
}

.changelog-detail p {
  margin: 0 0 8px;
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}

.changelog-detail ul {
  display: grid;
  grid-template-columns: repeat(2, minmax(260px, 1fr));
  gap: 6px 18px;
  margin: 0;
  padding-left: 18px;
  color: var(--el-text-color-regular);
  line-height: 1.6;
}

.credits {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  margin-top: 20px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

@media (max-width: 900px) {
  .intro-panel {
    grid-template-columns: 1fr;
    padding: 22px;
  }

  .changelog-title-row {
    grid-template-columns: 72px 96px minmax(0, 1fr);
  }

  .log-status {
    display: none;
  }

  .changelog-detail {
    padding-left: 0;
  }

  .changelog-detail ul {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 520px) {
  .intro-copy h1 {
    font-size: 26px;
  }

  .product-summary {
    font-size: 15px;
  }

  .changelog-title-row {
    grid-template-columns: 70px minmax(0, 1fr);
    gap: 8px;
  }

  .log-date {
    display: none;
  }
}
</style>
