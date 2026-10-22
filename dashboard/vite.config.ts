import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// `firewatch watch` serves the API on 127.0.0.1:7070 (see docs/API.md).
// The proxy keeps the app on relative `/api` URLs, which also work when the
// daemon serves the built dashboard itself via `dashboard_dir`.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:7070',
    },
  },
})
