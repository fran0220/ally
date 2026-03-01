import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import enApiConfig from '../../../messages/en/apiConfig.json';
import enAssetHub from '../../../messages/en/assetHub.json';
import enAuth from '../../../messages/en/auth.json';
import enCommon from '../../../messages/en/common.json';
import enLanding from '../../../messages/en/landing.json';
import enNav from '../../../messages/en/nav.json';
import enProfile from '../../../messages/en/profile.json';
import enStages from '../../../messages/en/stages.json';
import enWorkspace from '../../../messages/en/workspace.json';
import enWorkspaceDetail from '../../../messages/en/workspaceDetail.json';
import zhApiConfig from '../../../messages/zh/apiConfig.json';
import zhAssetHub from '../../../messages/zh/assetHub.json';
import zhAuth from '../../../messages/zh/auth.json';
import zhCommon from '../../../messages/zh/common.json';
import zhLanding from '../../../messages/zh/landing.json';
import zhNav from '../../../messages/zh/nav.json';
import zhProfile from '../../../messages/zh/profile.json';
import zhStages from '../../../messages/zh/stages.json';
import zhWorkspace from '../../../messages/zh/workspace.json';
import zhWorkspaceDetail from '../../../messages/zh/workspaceDetail.json';

const resources = {
  zh: {
    common: zhCommon,
    nav: zhNav,
    landing: zhLanding,
    auth: zhAuth,
    workspace: zhWorkspace,
    workspaceDetail: zhWorkspaceDetail,
    assetHub: zhAssetHub,
    stages: zhStages,
    apiConfig: zhApiConfig,
    profile: zhProfile,
  },
  en: {
    common: enCommon,
    nav: enNav,
    landing: enLanding,
    auth: enAuth,
    workspace: enWorkspace,
    workspaceDetail: enWorkspaceDetail,
    assetHub: enAssetHub,
    stages: enStages,
    apiConfig: enApiConfig,
    profile: enProfile,
  },
} as const;

const namespaces = [
  'common',
  'nav',
  'landing',
  'auth',
  'workspace',
  'workspaceDetail',
  'assetHub',
  'stages',
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
