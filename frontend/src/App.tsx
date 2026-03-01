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
const AiConfig = lazy(() => import('./routes/admin/AiConfig').then((module) => ({ default: module.AiConfig })));

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
        path: '/auth/signin',
        element: <SignIn />,
      },
      {
        path: '/auth/signup',
        element: <SignUp />,
      },
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
      },
      {
        path: '/admin/ai-config',
        element: <AiConfig />,
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
