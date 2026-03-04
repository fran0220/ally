import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import path from 'node:path';

const devApiProxyTarget = process.env.VITE_DEV_API_PROXY_TARGET ?? 'http://127.0.0.1:43001';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      'next-intl': path.resolve(__dirname, 'src/compat/next-intl.ts'),
      'next/navigation': path.resolve(__dirname, 'src/compat/next-navigation.ts'),
      'next/link': path.resolve(__dirname, 'src/compat/NextLink.tsx'),
    },
  },
  server: {
    host: '0.0.0.0',
    port: 45173,
    proxy: {
      '/api': {
        target: devApiProxyTarget,
        changeOrigin: true,
      },
      '/healthz': {
        target: devApiProxyTarget,
        changeOrigin: true,
      },
    },
  },
});
