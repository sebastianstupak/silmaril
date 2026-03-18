import './app.css';
import App from './App.svelte';
import { mount } from 'svelte';

try {
  const app = mount(App, {
    target: document.getElementById('app')!,
  });
} catch (e) {
  console.error('[silmaril-editor] Failed to mount:', e);
  const el = document.getElementById('app');
  if (el) {
    el.innerHTML = `<div style="color:#f44336;padding:20px;font-family:monospace;">
      <h2>Mount Error</h2>
      <pre>${e}</pre>
    </div>`;
  }
}
