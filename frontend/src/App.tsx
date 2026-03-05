import { Suspense, lazy, useEffect } from 'react';
import {
  Link,
  Navigate,
  Outlet,
  RouterProvider,
  createBrowserRouter,
  useLocation,
  useParams,
} from 'react-router-dom';

import { Navbar } from './components/Navbar';
import { useHasAuthToken } from './hooks/useHasAuthToken';
import i18n from './i18n';

const Landing = lazy(() => import('./routes/Landing').then((module) => ({ default: module.Landing })));
const SignIn = lazy(() => import('./routes/auth/SignIn').then((module) => ({ default: module.SignIn })));
const SignUp = lazy(() => import('./routes/auth/SignUp').then((module) => ({ default: module.SignUp })));
const WorkspaceList = lazy(() =>
  import('./routes/workspace/WorkspaceList').then((module) => ({ default: module.WorkspaceList })),
);
const ProjectWorkbench = lazy(() =>
  import('./routes/workspace/ProjectWorkbench').then((module) => ({ default: module.ProjectWorkbench })),
);
const AssetHub = lazy(() => import('./routes/workspace/AssetHub').then((module) => ({ default: module.AssetHub })));
const Profile = lazy(() => import('./routes/profile/Profile').then((module) => ({ default: module.Profile })));
const ProfileAccount = lazy(() => import('./routes/profile/AccountTab').then((module) => ({ default: module.AccountTab })));
const ProfileApiConfig = lazy(() =>
  import('./routes/profile/ProfileApiConfig').then((module) => ({ default: module.ProfileApiConfig })),
);
const ProfilePreferences = lazy(() =>
  import('./routes/profile/ProfilePreferences').then((module) => ({ default: module.ProfilePreferences })),
);
const AdminLayout = lazy(() => import('./routes/admin/AdminLayout').then((module) => ({ default: module.AdminLayout })));
const AiConfig = lazy(() => import('./routes/admin/AiConfig').then((module) => ({ default: module.AiConfig })));
const AdminUsers = lazy(() => import('./routes/admin/AdminUsers').then((module) => ({ default: module.AdminUsers })));
const AdminSystem = lazy(() => import('./routes/admin/AdminSystem').then((module) => ({ default: module.AdminSystem })));

function RouteFallback() {
  return (
    <main className="page-shell py-10">
      <div className="glass-surface text-sm text-[var(--glass-text-secondary)]">Loading page...</div>
    </main>
  );
}

function AppLayout() {
  return (
    <>
      <Navbar />
      <Suspense fallback={<RouteFallback />}>
        <Outlet />
      </Suspense>
    </>
  );
}

function RequireAuth() {
  const hasToken = useHasAuthToken();
  const location = useLocation();

  if (!hasToken) {
    return (
      <Navigate
        to="/auth/signin"
        replace
        state={{ from: `${location.pathname}${location.search}${location.hash}` }}
      />
    );
  }

  return <Outlet />;
}

function RedirectIfAuthenticated() {
  const hasToken = useHasAuthToken();

  if (hasToken) {
    return <Navigate to="/workspace" replace />;
  }

  return <Outlet />;
}

function NotFound() {
  return (
    <main className="page-shell py-10">
      <div className="glass-surface max-w-xl space-y-3">
        <h1 className="text-lg font-semibold text-[var(--glass-text-primary)]">Page not found</h1>
        <p className="text-sm text-[var(--glass-text-secondary)]">
          The page you requested does not exist in the migrated frontend routes.
        </p>
        <Link className="glass-btn-base glass-btn-primary inline-flex h-10 items-center px-4" to="/">
          Back to home
        </Link>
      </div>
    </main>
  );
}

function LocaleRedirect() {
  const { locale } = useParams<{ locale: string }>();
  const location = useLocation();
  const normalizedLocale = (locale ?? '').toLowerCase();
  const supported = normalizedLocale === 'zh' || normalizedLocale === 'en';

  useEffect(() => {
    if (supported && i18n.language !== normalizedLocale) {
      void i18n.changeLanguage(normalizedLocale);
    }
  }, [normalizedLocale, supported]);

  if (!supported) {
    return <NotFound />;
  }

  const parts = location.pathname.split('/').filter(Boolean);
  const pathWithoutLocale = parts.slice(1).join('/');
  const targetPath = pathWithoutLocale ? `/${pathWithoutLocale}` : '/';
  const target = `${targetPath}${location.search}${location.hash}`;

  return <Navigate to={target} replace />;
}

const router = createBrowserRouter([
  {
    element: <AppLayout />,
    children: [
      {
        path: '/',
        element: <Landing />,
      },
      {
        element: <RedirectIfAuthenticated />,
        children: [
          {
            path: '/auth/signin',
            element: <SignIn />,
          },
          {
            path: '/auth/signup',
            element: <SignUp />,
          },
        ],
      },
      {
        element: <RequireAuth />,
        children: [
          {
            path: '/workspace',
            element: <WorkspaceList />,
          },
          {
            path: '/workspace/:projectId',
            element: <ProjectWorkbench />,
          },
          {
            path: '/workspace/asset-hub',
            element: <AssetHub />,
          },
          {
            path: '/profile',
            element: <Profile />,
            children: [
              {
                index: true,
                element: <Navigate to="api-config" replace />,
              },
              {
                path: 'account',
                element: <ProfileAccount />,
              },
              {
                path: 'api-config',
                element: <ProfileApiConfig />,
              },
              {
                path: 'preferences',
                element: <ProfilePreferences />,
              },
            ],
          },
          {
            path: '/admin',
            element: <AdminLayout />,
            children: [
              {
                index: true,
                element: <Navigate to="ai-config" replace />,
              },
              {
                path: 'ai-config',
                element: <AiConfig />,
              },
              {
                path: 'users',
                element: <AdminUsers />,
              },
              {
                path: 'system',
                element: <AdminSystem />,
              },
            ],
          },
        ],
      },
      {
        path: '/:locale/*',
        element: <LocaleRedirect />,
      },
      {
        path: '*',
        element: <NotFound />,
      },
    ],
  },
]);

export function App() {
  return <RouterProvider router={router} />;
}
