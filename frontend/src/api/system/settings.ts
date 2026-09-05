import request from '@/utils/request'
import type {
  MailSetting,
  MailTestRequest,
  MailTestResponse,
  PaymentSetting,
  PaymentTestRequest,
  PaymentTestResponse,
  SiteSetting,
  SystemSettingData,
  SystemSettingResult
} from '@/types'

export function getSystemSettings(): SystemSettingResult<SystemSettingData> {
  return request({
    url: '/system/setting',
    method: 'get'
  })
}

export function updateSiteSetting(data: SiteSetting): SystemSettingResult<void> {
  return request({
    url: '/system/setting/site',
    method: 'put',
    data
  })
}

export function updatePaymentSetting(data: PaymentSetting): SystemSettingResult<void> {
  return request({
    url: '/system/setting/payment',
    method: 'put',
    data
  })
}

export function updateMailSetting(data: MailSetting): SystemSettingResult<void> {
  return request({
    url: '/system/setting/mail',
    method: 'put',
    data
  })
}

export function testPaymentSetting(data: PaymentTestRequest): SystemSettingResult<PaymentTestResponse> {
  return request({
    url: '/system/setting/payment/test',
    method: 'post',
    data
  })
}

export function testMailSetting(data: MailTestRequest): SystemSettingResult<MailTestResponse> {
  return request({
    url: '/system/setting/mail/test',
    method: 'post',
    data
  })
}
