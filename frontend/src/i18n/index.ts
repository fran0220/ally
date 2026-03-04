import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import enApiConfig from '../../../messages/en/apiConfig.json';
import enAssetModal from '../../../messages/en/assetModal.json';
import enAssetHub from '../../../messages/en/assetHub.json';
import enAssetPicker from '../../../messages/en/assetPicker.json';
import enAssets from '../../../messages/en/assets.json';
import enAuth from '../../../messages/en/auth.json';
import enCommon from '../../../messages/en/common.json';
import enLanding from '../../../messages/en/landing.json';
import enNav from '../../../messages/en/nav.json';
import enProviderSection from '../../../messages/en/providerSection.json';
import enProfile from '../../../messages/en/profile.json';
import enSmartImport from '../../../messages/en/smartImport.json';
import enStages from '../../../messages/en/stages.json';
import enVoice from '../../../messages/en/voice.json';
import enWorkspace from '../../../messages/en/workspace.json';
import enWorkspaceDetail from '../../../messages/en/workspaceDetail.json';
import zhApiConfig from '../../../messages/zh/apiConfig.json';
import zhAssetModal from '../../../messages/zh/assetModal.json';
import zhAssetHub from '../../../messages/zh/assetHub.json';
import zhAssetPicker from '../../../messages/zh/assetPicker.json';
import zhAssets from '../../../messages/zh/assets.json';
import zhAuth from '../../../messages/zh/auth.json';
import zhCommon from '../../../messages/zh/common.json';
import zhLanding from '../../../messages/zh/landing.json';
import zhNav from '../../../messages/zh/nav.json';
import zhProviderSection from '../../../messages/zh/providerSection.json';
import zhProfile from '../../../messages/zh/profile.json';
import zhSmartImport from '../../../messages/zh/smartImport.json';
import zhStages from '../../../messages/zh/stages.json';
import zhVoice from '../../../messages/zh/voice.json';
import zhWorkspace from '../../../messages/zh/workspace.json';
import zhWorkspaceDetail from '../../../messages/zh/workspaceDetail.json';

const zhCommonMerged = {
  ...zhCommon,
  assetModal: zhAssetModal,
  assets: zhAssets,
  assetPicker: zhAssetPicker,
};

const enCommonMerged = {
  ...enCommon,
  assetModal: enAssetModal,
  assets: enAssets,
  assetPicker: enAssetPicker,
};

const resources = {
  zh: {
    common: zhCommonMerged,
    nav: zhNav,
    providerSection: zhProviderSection,
    landing: zhLanding,
    auth: zhAuth,
    workspace: zhWorkspace,
    workspaceDetail: zhWorkspaceDetail,
    assetHub: zhAssetHub,
    smartImport: zhSmartImport,
    stages: zhStages,
    voice: zhVoice,
    apiConfig: zhApiConfig,
    profile: zhProfile,
  },
  en: {
    common: enCommonMerged,
    nav: enNav,
    providerSection: enProviderSection,
    landing: enLanding,
    auth: enAuth,
    workspace: enWorkspace,
    workspaceDetail: enWorkspaceDetail,
    assetHub: enAssetHub,
    smartImport: enSmartImport,
    stages: enStages,
    voice: enVoice,
    apiConfig: enApiConfig,
    profile: enProfile,
  },
} as const;

const namespaces = [
  'common',
  'nav',
  'providerSection',
  'landing',
  'auth',
  'workspace',
  'workspaceDetail',
  'assetHub',
  'smartImport',
  'stages',
  'voice',
  'apiConfig',
  'profile',
];

i18n.use(initReactI18next).init({
  resources,
  lng: navigator.language.toLowerCase().startsWith('zh') ? 'zh' : 'en',
  fallbackLng: 'en',
  defaultNS: 'common',
  ns: namespaces,
  fallbackNS: 'common',
  interpolation: { escapeValue: false },
});

export default i18n;
