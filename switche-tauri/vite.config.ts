import { defineConfig } from 'vite'

export default defineConfig({
  // we've setup mill to collate sjs output to webapp
  root: "./out/webapp",
  // prevent vite from obscuring rust errors
  clearScreen: false,
  // Tauri expects a fixed port, fail if that port is not available
  server: {
    host: '127.0.0.1',
    port: 4173,
    strictPort: true,
  },
  // to make use of `TAURI_PLATFORM`, `TAURI_ARCH`, `TAURI_FAMILY`,
  // `TAURI_PLATFORM_VERSION`, `TAURI_PLATFORM_TYPE` and `TAURI_DEBUG`
  // env variables
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // Tauri supports es2021
    //target: ['es2021', 'chrome97', 'safari13'],
    target: ['es2021', 'chrome97'],
    // don't minify for debug builds
    minify: !process.env.TAURI_DEBUG && 'esbuild',
    // produce sourcemaps for debug builds
    //sourcemap: !!process.env.TAURI_DEBUG,
  },
})
