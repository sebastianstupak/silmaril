<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';

  interface PermissionRequest {
    request_id: string;
    category: string;
    command_id: string;
  }

  let pendingRequest: PermissionRequest | null = $state(null);
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    unlisten = await listen<PermissionRequest>('ai:permission_request', (event) => {
      pendingRequest = event.payload;
    });
  });

  onDestroy(() => { unlisten?.(); });

  async function respond(level: 'once' | 'session' | 'always' | 'deny') {
    if (!pendingRequest) return;
    await invoke('ai_grant_permission', { request_id: pendingRequest.request_id, level });
    pendingRequest = null;
  }
</script>

{#if pendingRequest}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
    <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-6 max-w-sm w-full shadow-2xl">
      <h2 class="text-sm font-semibold text-zinc-100 mb-1">AI Permission Request</h2>
      <p class="text-xs text-zinc-400 mb-4">
        An AI agent wants to run <code class="text-orange-400">{pendingRequest.command_id}</code>
        (category: <span class="text-zinc-300">{pendingRequest.category}</span>).
      </p>
      <div class="flex flex-col gap-2">
        <button
          class="px-3 py-1.5 rounded bg-green-700 hover:bg-green-600 text-xs text-white"
          onclick={() => respond('once')}
        >Allow once</button>
        <button
          class="px-3 py-1.5 rounded bg-blue-700 hover:bg-blue-600 text-xs text-white"
          onclick={() => respond('session')}
        >Allow for session</button>
        <button
          class="px-3 py-1.5 rounded bg-blue-900 hover:bg-blue-800 text-xs text-white"
          onclick={() => respond('always')}
        >Always allow</button>
        <button
          class="px-3 py-1.5 rounded bg-zinc-700 hover:bg-zinc-600 text-xs text-zinc-200"
          onclick={() => respond('deny')}
        >Deny</button>
      </div>
    </div>
  </div>
{/if}
