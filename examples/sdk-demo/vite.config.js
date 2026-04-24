import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      '/api/v1/collect': {
        target: 'http://localhost:3456',
        changeOrigin: true,
      }
    }
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        about: resolve(__dirname, 'about.html'),
        detail: resolve(__dirname, 'detail.html'),
        dashboard: resolve(__dirname, 'dashboard.html'),
      }
    }
  }
});
