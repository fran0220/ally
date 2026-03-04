import type { SVGProps } from 'react';

import { customIcons } from './custom';

type IconAliasName = 'clapperboard' | 'link' | 'undo' | 'unplug';

const iconAliasMap: Record<IconAliasName, keyof typeof customIcons> = {
  clapperboard: 'video',
  link: 'externalLink',
  undo: 'refresh',
  unplug: 'wandOff',
};

export type AppIconName = keyof typeof customIcons | IconAliasName;

interface AppIconProps extends Omit<SVGProps<SVGSVGElement>, 'name'> {
  name: AppIconName;
  size?: number | string;
}

export function AppIcon({ name, size, ...props }: AppIconProps) {
  const resolvedName = (name in customIcons ? name : iconAliasMap[name as IconAliasName]) as keyof typeof customIcons;
  const IconComponent = customIcons[resolvedName] ?? customIcons.close;
  return <IconComponent width={size} height={size} {...props} />;
}
