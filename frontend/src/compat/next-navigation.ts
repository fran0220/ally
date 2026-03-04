import {
  useLocation,
  useNavigate,
  useParams,
  useSearchParams as useRouterSearchParams,
} from 'react-router-dom';

type NavigateOptions = {
  scroll?: boolean;
};

export function useRouter() {
  const navigate = useNavigate();

  return {
    push: (href: string) => {
      navigate(href);
    },
    replace: (href: string, _options?: NavigateOptions) => {
      navigate(href, { replace: true });
    },
    prefetch: async (_href: string) => {
      // React Router has no route prefetch API; keep this as a noop compatibility shim.
    },
  };
}

export function useSearchParams(): URLSearchParams {
  const [searchParams] = useRouterSearchParams();
  return searchParams;
}

export function usePathname(): string {
  const location = useLocation();
  return location.pathname;
}

export { useParams };
