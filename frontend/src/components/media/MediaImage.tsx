import type { CSSProperties, ImgHTMLAttributes, MouseEventHandler } from 'react';

export type MediaImageProps = {
  src: string | null | undefined;
  alt: string;
  className?: string;
  style?: CSSProperties;
  onClick?: MouseEventHandler<HTMLImageElement>;
  fill?: boolean;
  width?: number;
  height?: number;
  sizes?: string;
  priority?: boolean;
} & Omit<ImgHTMLAttributes<HTMLImageElement>, 'src' | 'alt' | 'width' | 'height'>;

export function MediaImage({
  src,
  alt,
  className,
  style,
  onClick,
  fill = false,
  width,
  height,
  sizes,
  priority = false,
  ...imgProps
}: MediaImageProps) {
  if (!src) {
    return null;
  }

  const mergedStyle: CSSProperties = {
    ...style,
    ...(fill ? { width: '100%', height: '100%' } : {}),
  };

  return (
    <img
      src={src}
      alt={alt}
      className={className}
      style={mergedStyle}
      onClick={onClick}
      loading={priority ? 'eager' : 'lazy'}
      width={fill ? undefined : width}
      height={fill ? undefined : height}
      sizes={sizes}
      {...imgProps}
    />
  );
}
