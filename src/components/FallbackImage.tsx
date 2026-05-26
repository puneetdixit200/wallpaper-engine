import { ReactNode, useEffect, useState } from "react";

interface FallbackImageProps {
  alt: string;
  fallback: ReactNode;
  src: string;
}

export function FallbackImage({ alt, fallback, src }: FallbackImageProps) {
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);
  }, [src]);

  if (!src || failed) {
    return <>{fallback}</>;
  }

  return <img alt={alt} onError={() => setFailed(true)} src={src} />;
}
