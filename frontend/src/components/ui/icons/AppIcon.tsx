import type { SVGProps } from 'react';

export type AppIconName = 'loader' | 'alert' | 'alertSolid' | 'info' | 'close' | 'mic' | 'checkSolid' | 'play' | 'pause' | 'chevronDown' | 'check' | 'checkDot' | 'edit' | 'trash' | 'sparklesAlt' | 'image' | 'search' | 'userAlt' | 'badgeCheck' | 'bolt';

interface AppIconProps extends Omit<SVGProps<SVGSVGElement>, 'name'> {
  name: AppIconName;
}

function baseProps(props: SVGProps<SVGSVGElement>): SVGProps<SVGSVGElement> {
  return {
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.8,
    strokeLinecap: 'round',
    strokeLinejoin: 'round',
    'aria-hidden': true,
    ...props,
  };
}

export function AppIcon({ name, ...props }: AppIconProps) {
  if (name === 'loader') {
    return (
      <svg {...baseProps(props)}>
        <circle cx="12" cy="12" r="8" opacity="0.25" />
        <path d="M20 12a8 8 0 0 0-8-8" />
      </svg>
    );
  }

  if (name === 'alert') {
    return (
      <svg {...baseProps(props)}>
        <path d="M12 3 2.6 19.2a1 1 0 0 0 .9 1.5h17a1 1 0 0 0 .9-1.5L12 3Z" />
        <path d="M12 9v5" />
        <circle cx="12" cy="17" r="1" fill="currentColor" stroke="none" />
      </svg>
    );
  }

  if (name === 'alertSolid') {
    return (
      <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
        <path d="M12 2.8 1.9 20.1a1.2 1.2 0 0 0 1 1.8h18.2a1.2 1.2 0 0 0 1-1.8L12 2.8Zm1.1 13.1h-2.2V9.2h2.2v6.7Zm-1.1 3.6a1.3 1.3 0 1 1 0-2.6 1.3 1.3 0 0 1 0 2.6Z" />
      </svg>
    );
  }

  if (name === 'info') {
    return (
      <svg {...baseProps(props)}>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 10v6" />
        <circle cx="12" cy="7" r="1" fill="currentColor" stroke="none" />
      </svg>
    );
  }

  if (name === 'mic') {
    return (
      <svg {...baseProps(props)}>
        <path d="M12 2a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
        <path d="M19 10v1a7 7 0 0 1-14 0v-1" />
        <path d="M12 19v3" />
      </svg>
    );
  }

  if (name === 'checkSolid') {
    return (
      <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
        <path d="M9 16.2 4.8 12l-1.4 1.4L9 19 21 7l-1.4-1.4L9 16.2Z" />
      </svg>
    );
  }

  if (name === 'play') {
    return (
      <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
        <path d="M8 5v14l11-7L8 5Z" />
      </svg>
    );
  }

  if (name === 'pause') {
    return (
      <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
        <path d="M6 4h4v16H6V4Zm8 0h4v16h-4V4Z" />
      </svg>
    );
  }

  if (name === 'chevronDown') {
    return (
      <svg {...baseProps(props)}>
        <path d="m6 9 6 6 6-6" />
      </svg>
    );
  }

  if (name === 'check') {
    return (
      <svg {...baseProps(props)}>
        <path d="M20 6 9 17l-5-5" />
      </svg>
    );
  }

  if (name === 'checkDot') {
    return (
      <svg {...baseProps(props)}>
        <path d="M20 6 9 17l-5-5" />
        <circle cx="12" cy="12" r="1" fill="currentColor" stroke="none" />
      </svg>
    );
  }

  if (name === 'edit') {
    return (
      <svg {...baseProps(props)}>
        <path d="M17 3a2.83 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
        <path d="m15 5 4 4" />
      </svg>
    );
  }

  if (name === 'trash') {
    return (
      <svg {...baseProps(props)}>
        <path d="M3 6h18" />
        <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
        <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
      </svg>
    );
  }

  if (name === 'sparklesAlt') {
    return (
      <svg {...baseProps(props)}>
        <path d="M12 3v2m0 14v2m9-9h-2M5 12H3m14.07-5.07-1.42 1.42M8.35 15.65l-1.42 1.42m11.14 0-1.42-1.42M8.35 8.35 6.93 6.93" />
        <circle cx="12" cy="12" r="3" />
      </svg>
    );
  }

  if (name === 'image') {
    return (
      <svg {...baseProps(props)}>
        <rect x="3" y="3" width="18" height="18" rx="2" />
        <circle cx="8.5" cy="8.5" r="1.5" />
        <path d="m21 15-5-5L5 21" />
      </svg>
    );
  }

  if (name === 'search') {
    return (
      <svg {...baseProps(props)}>
        <circle cx="11" cy="11" r="8" />
        <path d="m21 21-4.35-4.35" />
      </svg>
    );
  }

  if (name === 'userAlt') {
    return (
      <svg {...baseProps(props)}>
        <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
        <circle cx="12" cy="7" r="4" />
      </svg>
    );
  }

  if (name === 'badgeCheck') {
    return (
      <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
        <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm4.7 8.3-5.4 5.4a1 1 0 0 1-1.4 0l-2.6-2.6a1 1 0 1 1 1.4-1.4l1.9 1.9 4.7-4.7a1 1 0 1 1 1.4 1.4Z" />
      </svg>
    );
  }

  if (name === 'bolt') {
    return (
      <svg {...baseProps(props)}>
        <path d="M13 2 3 14h9l-1 8 10-12h-9l1-8Z" />
      </svg>
    );
  }

  return (
    <svg {...baseProps(props)}>
      <path d="m6 6 12 12" />
      <path d="M18 6 6 18" />
    </svg>
  );
}
