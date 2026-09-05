import type { AjaxResult } from "../common";

export interface SiteSetting {
  title: string;
  logo: string;
  favicon: string;
  description: string;
  keywords: string;
  frontendHeadCode: string;
  siteUrl: string;
  icpNo: string;
  publicSecurityNo: string;
  customerServiceEmail: string;
  copyright: string;
  defaultLanguage: string;
  enableSeo: boolean;
}

export interface PaymentSetting {
  enabled: boolean;
  provider: string;
  epayVersion: "v1" | "v2";
  gatewayUrl: string;
  merchantId: string;
  merchantKey: string;
  merchantPrivateKey: string;
  platformPublicKey: string;
  enabledPayTypes: string[];
  notifyUrl: string;
  returnUrl: string;
}

export interface MailSetting {
  enabled: boolean;
  provider: "custom" | "qq" | "gmail";
  host: string;
  port: number;
  username: string;
  password: string;
  fromEmail: string;
  fromName: string;
  encryption: "ssl" | "starttls" | "none";
  testRecipient: string;
}

export interface SystemSettingGeneratedUrls {
  notifyUrl: string;
  returnUrl: string;
}

export interface SystemSettingData {
  site: SiteSetting;
  payment: PaymentSetting;
  mail: MailSetting;
  generated: SystemSettingGeneratedUrls;
}

export interface PaymentTestRequest {
  action: string;
  notifyUrl?: string;
  returnUrl?: string;
}

export interface SystemConfigTestStep {
  name: string;
  status: "success" | "error" | "warning" | string;
  message: string;
  method?: string;
  url?: string;
  request?: Record<string, any>;
  response?: Record<string, any>;
  elapsedMs: number;
}

export interface PaymentTestResponse {
  success: boolean;
  action: string;
  version: string;
  outTradeNo: string;
  tradeNo: string;
  payType: string;
  payInfo: string;
  steps: SystemConfigTestStep[];
}

export interface MailTestRequest {
  to?: string;
  subject?: string;
  body?: string;
}

export interface MailTestResponse {
  success: boolean;
  message: string;
  server: string;
  elapsedMs: number;
}

export type SystemSettingResult<T> = Promise<AjaxResult<T>>;
