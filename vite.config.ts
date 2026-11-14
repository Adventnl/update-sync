import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: '127.0.0.1', port: 1420, strictPort: true,
    watch: { ignored: ['**/target/**', '**/src-tauri/**', '**/crates/**', '**/.impeccable/**', '**/.relay-*/**'] },
  },
  build: { target: ['es2022', 'chrome105', 'safari15'] },
});
