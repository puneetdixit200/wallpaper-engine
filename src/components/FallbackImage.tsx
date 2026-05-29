import { CSSProperties, ReactNode, useEffect, useState } from "react";

interface FallbackImageProps {
  alt: string;
  fallback: ReactNode;
  src: string;
  style?: CSSProperties;
}

export function FallbackImage({ alt, fallback, src, style }: FallbackImageProps) {
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);
  }, [src]);

  if (!src || failed) {
    return <>{fallback}</>;
  }

  return <img alt={alt} onError={() => setFailed(true)} src={src} style={style} />;
}
