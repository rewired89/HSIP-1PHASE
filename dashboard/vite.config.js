import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

<<<<<<< HEAD
// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
=======
export default defineConfig({
  plugins: [react()],
  server: {
    port: 3001,
    proxy: {
      '/v1': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      }
    }
  }
>>>>>>> origin/claude/fix-security-vulnerabilities-Kbv48
})
