<template>
  <div class="app-container system-settings">
    <el-alert title="支付和邮件操作使用已保存的配置。启用后可创建测试订单或发送测试邮件。" type="info" :closable="false" show-icon style="margin-bottom: 16px" />
    <el-tabs v-model="activeTab" class="settings-tabs">
      <el-tab-pane label="站点配置" name="site">
        <div class="setting-section">
          <el-form ref="siteFormRef" :model="siteForm" :rules="siteRules" label-width="140px">
            <el-row :gutter="20">
              <el-col :xs="24" :lg="12">
                <el-form-item label="站点标题" prop="title">
                  <el-input v-model="siteForm.title" maxlength="80" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="站点地址" prop="siteUrl">
                  <el-input v-model="siteForm.siteUrl" maxlength="200" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="站点Logo" prop="logo">
                  <image-upload v-model="siteForm.logo" :limit="1" :file-size="2" :file-type="['png', 'jpg', 'jpeg', 'webp']" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="站点图标" prop="favicon">
                  <div class="favicon-setting">
                    <el-upload
                      :action="faviconUploadUrl"
                      :headers="faviconUploadHeaders"
                      :show-file-list="false"
                      :before-upload="beforeFaviconUpload"
                      :on-success="handleFaviconUploadSuccess"
                      :on-error="handleFaviconUploadError"
                      accept=".png,image/png"
                    >
                      <div class="favicon-upload-card" :class="{ 'is-warning': siteForm.favicon && faviconLoadFailed }">
                        <div class="favicon-preview">
                          <img
                            v-if="siteForm.favicon && faviconPreviewUrl && !faviconLoadFailed"
                            :src="faviconPreviewUrl"
                            alt="站点图标预览"
                            @load="faviconLoadFailed = false"
                            @error="faviconLoadFailed = true"
                          />
                          <el-icon v-else><Picture /></el-icon>
                        </div>
                        <div class="favicon-meta">
                          <div class="favicon-title">{{ siteForm.favicon ? '当前站点图标' : '上传站点图标' }}</div>
                          <div class="favicon-path">{{ siteForm.favicon ? faviconDisplayName : '推荐 32x32 或 64x64' }}</div>
                        </div>
                        <span class="favicon-change">更换</span>
                      </div>
                    </el-upload>
                    <div class="favicon-tools">
                      <el-button link type="primary" @click="restoreDefaultFavicon">恢复默认</el-button>
                      <el-button v-if="siteForm.favicon" link type="danger" @click="clearFavicon">清除</el-button>
                      <el-tag v-if="siteForm.favicon && faviconLoadFailed" type="warning" effect="plain">预览异常</el-tag>
                    </div>
                    <div class="favicon-tip">请上传 png 文件，大小不超过 1MB。</div>
                  </div>
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="站点描述" prop="description">
                  <el-input v-model="siteForm.description" type="textarea" :rows="3" maxlength="300" show-word-limit />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="站点关键词" prop="keywords">
                  <el-input v-model="siteForm.keywords" type="textarea" :rows="3" maxlength="300" show-word-limit />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="ICP备案号" prop="icpNo">
                  <el-input v-model="siteForm.icpNo" maxlength="80" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="公网安备号" prop="publicSecurityNo">
                  <el-input v-model="siteForm.publicSecurityNo" maxlength="80" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="客服邮箱" prop="customerServiceEmail">
                  <el-input v-model="siteForm.customerServiceEmail" maxlength="120" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="默认语言" prop="defaultLanguage">
                  <el-input v-model="siteForm.defaultLanguage" maxlength="20" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="SEO开关" prop="enableSeo">
                  <el-switch v-model="siteForm.enableSeo" />
                </el-form-item>
              </el-col>
              <el-col :xs="24">
                <el-form-item label="版权文本" prop="copyright">
                  <el-input v-model="siteForm.copyright" maxlength="200" />
                </el-form-item>
              </el-col>
              <el-col :xs="24">
                <el-form-item label="前端头部代码" prop="frontendHeadCode">
                  <el-input v-model="siteForm.frontendHeadCode" type="textarea" :rows="7" disabled placeholder="兼容版不执行自定义头部脚本" />
                </el-form-item>
              </el-col>
            </el-row>
            <div class="form-actions">
              <el-button type="primary" icon="Check" :loading="saving.site" @click="submitSite" v-hasPermi="['system:setting:edit']">保存站点配置</el-button>
            </div>
          </el-form>
        </div>
      </el-tab-pane>

      <el-tab-pane label="支付配置" name="payment">
        <div class="setting-section">
          <el-form ref="paymentFormRef" :model="paymentForm" :rules="paymentRules" label-width="130px" class="payment-form">
            <div class="payment-option-bar">
              <el-form-item label="启用支付" prop="enabled">
                <el-switch v-model="paymentForm.enabled" />
              </el-form-item>
              <el-form-item label="接口方式" prop="epayVersion">
                <el-radio-group v-model="paymentForm.epayVersion">
                  <el-radio value="v1">易支付V1</el-radio>
                  <el-radio value="v2">易支付V2</el-radio>
                </el-radio-group>
              </el-form-item>
              <el-form-item label="启用方式" prop="enabledPayTypes">
                <el-checkbox-group v-model="paymentForm.enabledPayTypes">
                  <el-checkbox value="alipay">支付宝支付</el-checkbox>
                  <el-checkbox value="wxpay">微信支付</el-checkbox>
                </el-checkbox-group>
              </el-form-item>
            </div>
            <el-row :gutter="20">
              <el-col :xs="24">
                <el-form-item label="支付网关地址" prop="gatewayUrl">
                  <el-input v-model="paymentForm.gatewayUrl" placeholder="https://pay.example.com" maxlength="200" />
                </el-form-item>
              </el-col>
              <el-col :xs="24">
                <el-form-item label="商户ID" prop="merchantId">
                  <el-input v-model="paymentForm.merchantId" maxlength="80" />
                </el-form-item>
              </el-col>
              <el-col v-if="paymentForm.epayVersion === 'v1'" :xs="24">
                <el-form-item label="商户密钥" prop="merchantKey">
                  <el-input v-model="paymentForm.merchantKey" type="password" show-password />
                </el-form-item>
              </el-col>
              <el-col v-if="paymentForm.epayVersion === 'v2'" :xs="24" :lg="12">
                <el-form-item label="商户私钥" prop="merchantPrivateKey">
                  <el-input v-model="paymentForm.merchantPrivateKey" type="textarea" :rows="8" />
                </el-form-item>
              </el-col>
              <el-col v-if="paymentForm.epayVersion === 'v2'" :xs="24" :lg="12">
                <el-form-item label="平台公钥" prop="platformPublicKey">
                  <el-input v-model="paymentForm.platformPublicKey" type="textarea" :rows="8" />
                </el-form-item>
              </el-col>
              <el-col :xs="24">
                <el-form-item label="异步通知地址" prop="notifyUrl">
                  <el-input v-model="paymentForm.notifyUrl" :placeholder="generated.notifyUrl">
                    <template #append>
                      <el-button icon="DocumentCopy" @click="paymentForm.notifyUrl = generated.notifyUrl">填入默认</el-button>
                    </template>
                  </el-input>
                </el-form-item>
              </el-col>
              <el-col :xs="24">
                <el-form-item label="跳转通知地址" prop="returnUrl">
                  <el-input v-model="paymentForm.returnUrl" :placeholder="generated.returnUrl">
                    <template #append>
                      <el-button icon="DocumentCopy" @click="paymentForm.returnUrl = generated.returnUrl">填入默认</el-button>
                    </template>
                  </el-input>
                </el-form-item>
              </el-col>
            </el-row>
            <div class="form-actions payment-actions">
              <el-button type="primary" icon="Check" :loading="saving.payment" @click="submitPayment" v-hasPermi="['system:setting:edit']">保存支付配置</el-button>
              <el-button icon="Connection" :loading="paymentTesting" @click="runPaymentTest()" v-hasPermi="['system:setting:pay:test']">创建 0.01 元测试订单</el-button>
            </div>
          </el-form>
        </div>

        <div v-if="paymentResult" class="setting-section result-section">
          <div class="result-heading">
            <span>支付功能测试结果</span>
            <el-tag :type="paymentResult.success ? 'success' : 'danger'" effect="plain">{{ paymentResult.success ? '通过' : '未通过' }}</el-tag>
          </div>
          <el-descriptions :column="3" border class="result-summary">
            <el-descriptions-item label="接口版本">{{ paymentResult.version }}</el-descriptions-item>
            <el-descriptions-item label="商户订单号">{{ paymentResult.outTradeNo }}</el-descriptions-item>
            <el-descriptions-item label="平台订单号">{{ paymentResult.tradeNo || '-' }}</el-descriptions-item>
            <el-descriptions-item label="支付方式">{{ paymentResult.payType || '-' }}</el-descriptions-item>
            <el-descriptions-item label="发起参数" :span="2">{{ paymentResult.payInfo || '-' }}</el-descriptions-item>
          </el-descriptions>
          <el-table :data="paymentResult.steps" border class="result-table">
            <el-table-column label="步骤" prop="name" width="150" />
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="stepTagType(row.status)" effect="plain">{{ stepStatusText(row.status) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="耗时" width="100">
              <template #default="{ row }">{{ row.elapsedMs }}ms</template>
            </el-table-column>
            <el-table-column label="结果" prop="message" min-width="180" />
            <el-table-column label="详情" min-width="260">
              <template #default="{ row }">
                <el-collapse v-if="row.request || row.response">
                  <el-collapse-item title="请求与响应">
                    <pre class="json-preview">{{ formatJson({ url: row.url, method: row.method, request: row.request, response: row.response }) }}</pre>
                  </el-collapse-item>
                </el-collapse>
              </template>
            </el-table-column>
          </el-table>
        </div>
        <div class="setting-section">
          <div class="section-heading"><h3>最近支付订单</h3><el-button :loading="ordersLoading" @click="loadOrders">刷新订单</el-button></div>
          <el-table :data="paymentOrders" style="width: 100%">
            <el-table-column label="商户订单号" prop="outTradeNo" min-width="260" />
            <el-table-column label="金额（元）" width="110"><template #default="{ row }">{{ (row.amountCents / 100).toFixed(2) }}</template></el-table-column>
            <el-table-column label="状态" width="110"><template #default="{ row }"><el-tag :type="row.status === 'paid' ? 'success' : 'info'">{{ row.status === 'paid' ? '已支付' : '待确认' }}</el-tag></template></el-table-column>
            <el-table-column label="创建时间" prop="createTime" min-width="180" />
            <el-table-column label="操作" width="120"><template #default="{ row }"><el-button link type="primary" :loading="queryingOrder === row.outTradeNo" @click="queryOrder(row.outTradeNo)" v-hasPermi="['system:setting:pay:test']">查询网关</el-button></template></el-table-column>
          </el-table>
        </div>
      </el-tab-pane>

      <el-tab-pane label="邮箱配置" name="mail">
        <div class="setting-section">
          <el-form ref="mailFormRef" :model="mailForm" :rules="mailRules" label-width="140px">
            <el-row :gutter="20">
              <el-col :xs="24" :lg="8">
                <el-form-item label="启用发信" prop="enabled">
                  <el-switch v-model="mailForm.enabled" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="8">
                <el-form-item label="邮箱类型" prop="provider">
                  <el-select v-model="mailForm.provider" style="width: 100%" @change="handleMailProviderChange">
                    <el-option label="自定义SMTP" value="custom" />
                    <el-option label="QQ邮箱" value="qq" />
                    <el-option label="谷歌邮箱" value="gmail" />
                  </el-select>
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="8">
                <el-form-item label="加密方式" prop="encryption">
                  <el-select v-model="mailForm.encryption" style="width: 100%">
                    <el-option label="SSL/TLS" value="ssl" />
                    <el-option label="STARTTLS" value="starttls" />
                    <el-option label="不加密" value="none" />
                  </el-select>
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="SMTP服务器" prop="host">
                  <el-input v-model="mailForm.host" maxlength="120" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="SMTP端口" prop="port">
                  <el-input-number v-model="mailForm.port" :min="1" :max="65535" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="SMTP账号" prop="username">
                  <el-input v-model="mailForm.username" maxlength="120" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="授权码/密码" prop="password">
                  <el-input v-model="mailForm.password" type="password" show-password />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="发件邮箱" prop="fromEmail">
                  <el-input v-model="mailForm.fromEmail" maxlength="120" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="发件名称" prop="fromName">
                  <el-input v-model="mailForm.fromName" maxlength="80" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :lg="12">
                <el-form-item label="测试收件人" prop="testRecipient">
                  <el-input v-model="mailForm.testRecipient" maxlength="120" />
                </el-form-item>
              </el-col>
            </el-row>
            <div class="form-actions">
              <el-button type="primary" icon="Check" :loading="saving.mail" @click="submitMail" v-hasPermi="['system:setting:edit']">保存邮箱配置</el-button>
              <el-button icon="Message" :loading="mailTesting" @click="runMailTest" v-hasPermi="['system:setting:mail:test']">发送测试邮件</el-button>
            </div>
          </el-form>
        </div>

        <div v-if="mailResult" class="setting-section result-section">
          <div class="result-heading">
            <span>邮箱测试结果</span>
            <el-tag :type="mailResult.success ? 'success' : 'danger'" effect="plain">{{ mailResult.success ? '通过' : '未通过' }}</el-tag>
          </div>
          <el-descriptions :column="3" border>
            <el-descriptions-item label="SMTP服务器">{{ mailResult.server }}</el-descriptions-item>
            <el-descriptions-item label="耗时">{{ mailResult.elapsedMs }}ms</el-descriptions-item>
            <el-descriptions-item label="结果">{{ mailResult.message }}</el-descriptions-item>
          </el-descriptions>
        </div>
      </el-tab-pane>
    </el-tabs>

  </div>
</template>

<script setup lang="ts" name="SystemSettings">
import type {
  MailSetting,
  MailTestResponse,
  PaymentSetting,
  PaymentTestResponse,
  SiteSetting,
  SystemConfigTestStep,
  SystemSettingGeneratedUrls
} from '@/types'
import {
  getSystemSettings,
  testMailSetting,
  testPaymentSetting,
  updateMailSetting,
  updatePaymentSetting,
  updateSiteSetting
} from '@/api/system/settings'
import { getToken } from '@/utils/auth'
import request from '@/utils/request'
import type { UploadFileResult } from '@/types/api/common'

const { proxy } = getCurrentInstance() as any

const activeTab = ref('site')
const loading = ref(false)
const paymentOrders = ref<any[]>([])
const ordersLoading = ref(false)
const queryingOrder = ref('')
async function loadOrders() {
  ordersLoading.value = true
  try { const response: any = await request({ url: '/payment/orders', method: 'get' }); paymentOrders.value = response.rows } finally { ordersLoading.value = false }
}
async function queryOrder(number: string) {
  queryingOrder.value = number
  try { await request({ url: '/payment/orders/' + encodeURIComponent(number) + '/query', method: 'post' }); await loadOrders() } finally { queryingOrder.value = '' }
}
const paymentTesting = ref(false)
const mailTesting = ref(false)
const saving = reactive({ site: false, payment: false, mail: false })

const siteFormRef = ref()
const paymentFormRef = ref()
const mailFormRef = ref()

const generated = reactive<SystemSettingGeneratedUrls>({
  notifyUrl: '',
  returnUrl: ''
})

const siteForm = reactive<SiteSetting>(defaultSiteSetting())
const paymentForm = reactive<PaymentSetting>(defaultPaymentSetting())
const mailForm = reactive<MailSetting>(defaultMailSetting())

const paymentResult = ref<PaymentTestResponse>()
const mailResult = ref<MailTestResponse>()
const faviconLoadFailed = ref(false)

const faviconUploadUrl = computed(() => `${import.meta.env.VITE_APP_BASE_API}/common/upload`)
const faviconUploadHeaders = computed(() => ({ Authorization: `Bearer ${getToken()}` }))
const faviconPreviewUrl = computed(() => resolveFaviconUrl(siteForm.favicon))
const faviconDisplayName = computed(() => getFaviconDisplayName(siteForm.favicon))

const siteRules = {
  title: [{ required: true, message: '站点标题不能为空', trigger: 'blur' }],
  customerServiceEmail: [{ type: 'email', message: '请输入正确的邮箱地址', trigger: ['blur', 'change'] }]
}

const paymentRules = {
  gatewayUrl: [{ required: true, message: '支付网关地址不能为空', trigger: 'blur' }],
  merchantId: [{ required: true, message: '商户ID不能为空', trigger: 'blur' }],
  enabledPayTypes: [{ type: 'array', required: true, message: '请至少启用一种支付方式', trigger: 'change' }],
  merchantKey: [{ required: true, message: '商户密钥不能为空', trigger: 'blur' }],
  merchantPrivateKey: [{ required: true, message: '商户私钥不能为空', trigger: 'blur' }]
}

const mailRules = {
  host: [{ required: true, message: 'SMTP服务器不能为空', trigger: 'blur' }],
  port: [{ required: true, message: 'SMTP端口不能为空', trigger: 'blur' }],
  username: [{ required: true, message: 'SMTP账号不能为空', trigger: 'blur' }],
  password: [{ required: true, message: '授权码/密码不能为空', trigger: 'blur' }],
  fromEmail: [{ type: 'email', message: '请输入正确的邮箱地址', trigger: ['blur', 'change'] }],
  testRecipient: [{ type: 'email', message: '请输入正确的邮箱地址', trigger: ['blur', 'change'] }]
}

function defaultSiteSetting(): SiteSetting {
  return {
    title: 'RuoYi-Rust BY',
    logo: '',
    favicon: '/admin/favicon.ico',
    description: '',
    keywords: '',
    frontendHeadCode: '',
    siteUrl: '',
    icpNo: '',
    publicSecurityNo: '',
    customerServiceEmail: '',
    copyright: 'Copyright © 2026 RuoYi-Rust BY. All Rights Reserved.',
    defaultLanguage: 'zh-CN',
    enableSeo: true
  }
}

function defaultPaymentSetting(): PaymentSetting {
  return {
    enabled: false,
    provider: 'epay',
    epayVersion: 'v1',
    gatewayUrl: '',
    merchantId: '',
    merchantKey: '',
    merchantPrivateKey: '',
    platformPublicKey: '',
    enabledPayTypes: ['alipay'],
    notifyUrl: '',
    returnUrl: ''
  }
}

function defaultMailSetting(): MailSetting {
  return {
    enabled: false,
    provider: 'custom',
    host: '',
    port: 465,
    username: '',
    password: '',
    fromEmail: '',
    fromName: 'RuoYi-Rust BY',
    encryption: 'ssl',
    testRecipient: ''
  }
}

function loadSettings() {
  loading.value = true
  getSystemSettings().then(response => {
    Object.assign(siteForm, defaultSiteSetting(), response.data?.site)
    Object.assign(paymentForm, defaultPaymentSetting(), response.data?.payment)
    Object.assign(mailForm, defaultMailSetting(), response.data?.mail)
    Object.assign(generated, response.data?.generated)
  }).finally(() => {
    loading.value = false
  })
}

function submitSite() {
  siteFormRef.value?.validate((valid: boolean) => {
    if (!valid) return
    saving.site = true
    updateSiteSetting(siteForm).then(() => {
      proxy.$modal.msgSuccess('保存成功')
      loadSettings()
    }).finally(() => {
      saving.site = false
    })
  })
}

function submitPayment() {
  paymentFormRef.value?.validate((valid: boolean) => {
    if (!valid) return
    saving.payment = true
    updatePaymentSetting(paymentForm).then(() => {
      proxy.$modal.msgSuccess('保存成功')
      loadSettings()
    }).finally(() => {
      saving.payment = false
    })
  })
}

function submitMail() {
  mailFormRef.value?.validate((valid: boolean) => {
    if (!valid) return
    saving.mail = true
    updateMailSetting(mailForm).then(() => {
      proxy.$modal.msgSuccess('保存成功')
      loadSettings()
    }).finally(() => {
      saving.mail = false
    })
  })
}

async function runPaymentTest() {
  try { await proxy.$modal.confirm("将使用已保存的配置创建真实的 0.01 元订单。创建订单不会自动扣款，是否继续？") } catch { return }
  paymentTesting.value = true
  paymentResult.value = undefined
  testPaymentSetting({
    action: 'function',
    notifyUrl: paymentForm.notifyUrl || generated.notifyUrl,
    returnUrl: paymentForm.returnUrl || generated.returnUrl
  }).then(response => {
    paymentResult.value = response.data
    loadOrders()
    if (response.data?.success) {
      proxy.$modal.msgSuccess('测试订单已创建，付款后以订单状态为准')
    } else {
      proxy.$modal.msgWarning('测试未通过')
    }
  }).finally(() => {
    paymentTesting.value = false
  })
}

function runMailTest() {
  mailTesting.value = true
  mailResult.value = undefined
  testMailSetting({
    to: mailForm.testRecipient,
    subject: 'RuoYi-Rust BY邮箱测试',
    body: '这是一封来自 RuoYi-Rust BY 系统配置的 SMTP 测试邮件。'
  }).then(response => {
    mailResult.value = response.data
    if (response.data?.success) {
      proxy.$modal.msgSuccess('邮件已发送')
    } else {
      proxy.$modal.msgWarning(response.data?.message || '邮件测试未通过')
    }
  }).finally(() => {
    mailTesting.value = false
  })
}

function handleMailProviderChange() {
  if (mailForm.provider === 'qq') {
    mailForm.host = 'smtp.qq.com'
    mailForm.port = 465
    mailForm.encryption = 'ssl'
  } else if (mailForm.provider === 'gmail') {
    mailForm.host = 'smtp.gmail.com'
    mailForm.port = 587
    mailForm.encryption = 'starttls'
  }
}

function stepTagType(status: SystemConfigTestStep['status']) {
  if (status === 'success') return 'success'
  if (status === 'warning') return 'warning'
  return 'danger'
}

function stepStatusText(status: SystemConfigTestStep['status']) {
  if (status === 'success') return '成功'
  if (status === 'warning') return '警告'
  return '失败'
}

function formatJson(value: any) {
  return JSON.stringify(value, null, 2)
}

watch(() => siteForm.favicon, () => {
  faviconLoadFailed.value = false
})

function beforeFaviconUpload(file: File) {
  const extension = file.name.includes('.') ? file.name.split('.').pop()?.toLowerCase() : ''
  if (!extension || !['png'].includes(extension)) {
    proxy.$modal.msgError('文件格式不正确，请上传 png 格式的站点图标')
    return false
  }
  if (file.name.includes(',')) {
    proxy.$modal.msgError('文件名不能包含英文逗号')
    return false
  }
  if (file.size / 1024 / 1024 > 1) {
    proxy.$modal.msgError('站点图标大小不能超过 1MB')
    return false
  }
  proxy.$modal.loading('正在上传站点图标，请稍候...')
  return true
}

function handleFaviconUploadSuccess(res: UploadFileResult) {
  proxy.$modal.closeLoading()
  if (res.code !== 200) {
    proxy.$modal.msgError(res.msg || '站点图标上传失败')
    return
  }
  siteForm.favicon = normalizeUploadedFaviconUrl(res)
  faviconLoadFailed.value = false
  proxy.$modal.msgSuccess('站点图标已上传')
}

function handleFaviconUploadError() {
  proxy.$modal.closeLoading()
  proxy.$modal.msgError('站点图标上传失败')
}

function restoreDefaultFavicon() {
  siteForm.favicon = '/admin/favicon.ico'
}

function clearFavicon() {
  siteForm.favicon = ''
}

function normalizeUploadedFaviconUrl(res: UploadFileResult) {
  if (res.url) {
    try {
      const parsed = new URL(res.url, window.location.origin)
      if (parsed.pathname) {
        return `${parsed.pathname}${parsed.search}${parsed.hash}`
      }
    } catch {
      return res.url
    }
  }
  if (res.fileName) {
    return res.fileName.startsWith('/') ? res.fileName : `/${res.fileName}`
  }
  return ''
}

function resolveFaviconUrl(value: string) {
  const url = String(value || '').trim()
  if (!url) return ''
  if (/^(https?:)?\/\//i.test(url) || url.startsWith('data:') || url.startsWith('blob:')) {
    return url
  }
  if (url.startsWith('/')) {
    return url
  }
  return `/${url.replace(/^\.?\//, '')}`
}

function getFaviconDisplayName(value: string) {
  const url = String(value || '').trim()
  if (!url) return ''
  const cleanUrl = url.split('?')[0].split('#')[0]
  return cleanUrl.slice(cleanUrl.lastIndexOf('/') + 1) || cleanUrl
}

loadSettings()
</script>

<style scoped lang="scss">
.system-settings {
  .settings-tabs {
    background: #fff;
    border: 1px solid #e4e7ed;
    border-radius: 4px;
    padding: 0 16px 16px;
  }

  .setting-section {
    border: 1px solid #e4e7ed;
    border-radius: 4px;
    padding: 18px 18px 14px;
    margin-top: 14px;
    background: #fff;
  }

  .form-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
    border-top: 1px solid #ebeef5;
    padding-top: 14px;
  }

  .payment-option-bar {
    display: grid;
    grid-template-columns: 220px 380px 440px;
    gap: 10px 28px;
    align-items: start;
    max-width: 1160px;
    margin-bottom: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid #ebeef5;

    :deep(.el-form-item) {
      margin-bottom: 8px;
    }

    :deep(.el-form-item__content) {
      min-width: 0;
    }

    :deep(.el-radio),
    :deep(.el-checkbox) {
      margin-right: 22px;
    }
  }

  .payment-actions {
    justify-content: center;
  }

  .favicon-setting {
    width: 100%;
    max-width: 430px;

    :deep(.el-upload) {
      width: 100%;
      text-align: left;
    }
  }

  .favicon-upload-card {
    display: flex;
    align-items: center;
    width: 100%;
    min-height: 84px;
    padding: 12px 14px;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
    background: #fff;
    cursor: pointer;
    transition: border-color 0.2s, background-color 0.2s;

    &:hover {
      border-color: #409eff;
      background: #f8fbff;
    }

    &.is-warning {
      border-color: #e6a23c;
      background: #fffaf0;
    }
  }

  .favicon-preview {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 56px;
    width: 56px;
    height: 56px;
    border: 1px solid #ebeef5;
    border-radius: 4px;
    background:
      linear-gradient(45deg, #f6f7f9 25%, transparent 25%),
      linear-gradient(-45deg, #f6f7f9 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #f6f7f9 75%),
      linear-gradient(-45deg, transparent 75%, #f6f7f9 75%);
    background-color: #fff;
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
    background-size: 16px 16px;
    color: #909399;

    img {
      display: block;
      width: 38px;
      height: 38px;
      object-fit: contain;
    }

    .el-icon {
      font-size: 24px;
    }
  }

  .favicon-meta {
    flex: 1;
    min-width: 0;
    margin: 0 12px;
  }

  .favicon-title {
    color: #303133;
    font-size: 14px;
    font-weight: 600;
    line-height: 22px;
  }

  .favicon-path {
    overflow: hidden;
    margin-top: 4px;
    color: #909399;
    font-size: 12px;
    line-height: 18px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .favicon-change {
    flex: 0 0 auto;
    color: #409eff;
    font-size: 13px;
  }

  .favicon-tools {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 26px;
    margin-top: 6px;
  }

  .favicon-tip {
    color: #909399;
    font-size: 12px;
    line-height: 18px;
  }

  .result-section {
    .result-heading {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 12px;
      font-weight: 600;
    }

    .result-summary {
      margin-bottom: 12px;
    }

    .result-table {
      width: 100%;
    }
  }

  .json-preview {
    max-height: 260px;
    overflow: auto;
    margin: 0;
    padding: 10px;
    border-radius: 4px;
    background: #f6f8fa;
    color: #303133;
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }
}

@media (max-width: 768px) {
  .system-settings {
    .settings-tabs {
      padding: 0 10px 12px;
    }

    .setting-section {
      padding: 14px 10px 10px;
    }

    .form-actions {
      justify-content: flex-start;
    }

    .payment-option-bar {
      grid-template-columns: 1fr;
      gap: 0;
    }

    .payment-actions {
      justify-content: flex-start;
    }
  }
}

@media (min-width: 769px) and (max-width: 1200px) {
  .system-settings {
    .payment-option-bar {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      max-width: none;
    }
  }
}
</style>
