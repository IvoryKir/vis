import { ref, watch } from 'vue';
import { themes } from '../themes';

const STORAGE_KEY = 'vis_theme';

const currentTheme = ref<string>(localStorage.getItem(STORAGE_KEY) || 'default');

function applyTheme(themeId: string) {
  const theme = themes.find(t => t.id === themeId) || themes[0];
  const root = document.documentElement;
  
  for (const [key, value] of Object.entries(theme.colors)) {
    root.style.setProperty(`--${key}`, value);
  }
}

// Apply on load
applyTheme(currentTheme.value);

watch(currentTheme, (newTheme) => {
  localStorage.setItem(STORAGE_KEY, newTheme);
  applyTheme(newTheme);
});

export function useTheme() {
  return {
    currentTheme,
    themes,
    setTheme: (id: string) => {
      currentTheme.value = id;
    }
  };
}
