import { useEffect, useState, ReactNode } from 'react';

export function NoSSR({ children }: { children: ReactNode }) {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  if (!mounted) return <div style={{ height: 400 }} />;
  return <>{children}</>;
}
