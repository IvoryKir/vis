import { createApp } from 'vue';
import '@xterm/xterm/css/xterm.css';
import './styles/tailwind.css';
import App from './App.vue';

// Mark Tauri desktop for CSS optimizations (reduced shadows, etc.)
if ('__TAURI_INTERNALS__' in window) {
  document.documentElement.classList.add('tauri-desktop');
}

createApp(App).mount('#app');
