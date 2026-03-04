import { forwardRef, type ComponentProps } from 'react';
import { Link } from 'react-router-dom';

type RouterLinkProps = ComponentProps<typeof Link>;

type NextLinkProps = Omit<RouterLinkProps, 'to'> & {
  href: string;
  prefetch?: boolean;
};

const NextLink = forwardRef<HTMLAnchorElement, NextLinkProps>(function NextLink(
  { href, prefetch: _prefetch, ...props },
  ref,
) {
  return <Link ref={ref} to={href} {...props} />;
});

export default NextLink;
