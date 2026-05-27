<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

interface WidgetConfig {
  agent_url: string
  auth_token: string | null
  poll_interval_s: number
  autostart: boolean
  theme: 'light' | 'dark' | 'system'
}

const cfg = reactive<WidgetConfig>({
  agent_url: 'http://localhost:7844',
  auth_token: '',
  poll_interval_s: 30,
  autostart: true,
  theme: 'system',
})

const saving = ref(false)
const message = ref('')

onMounted(async () => {
  try {
    const remote = await invoke<WidgetConfig>('get_config')
    Object.assign(cfg, remote)
    if (cfg.auth_token === null) cfg.auth_token = ''
  } catch (err) {
    message.value = `Failed to load config: ${err}`
  }
})

async function save() {
  saving.value = true
  message.value = ''
  try {
    const payload: WidgetConfig = {
      ...cfg,
      auth_token: cfg.auth_token && cfg.auth_token.length > 0 ? cfg.auth_token : null,
    }
    await invoke('save_config', { newCfg: payload })
    message.value = 'Saved.'
  } catch (err) {
    message.value = `Save failed: ${err}`
  } finally {
    saving.value = false
  }
}

async function openDashboard() {
  await invoke('open_dashboard')
}

async function close() {
  await getCurrentWindow().hide()
}
</script>

<template>
  <main :data-theme="cfg.theme">
    <h1>Spoke Widget — Settings</h1>

    <form @submit.prevent="save">
      <label>
        <span>Spoke-Agent URL</span>
        <input v-model="cfg.agent_url" type="url" required />
      </label>

      <label>
        <span>Auth token (optional)</span>
        <input v-model="cfg.auth_token" type="password" placeholder="Bearer …" />
      </label>

      <label>
        <span>Polling interval</span>
        <select v-model.number="cfg.poll_interval_s">
          <option :value="10">10 s</option>
          <option :value="30">30 s</option>
          <option :value="60">60 s</option>
        </select>
      </label>

      <label class="row">
        <input v-model="cfg.autostart" type="checkbox" />
        <span>Start with system</span>
      </label>

      <label>
        <span>Theme</span>
        <select v-model="cfg.theme">
          <option value="light">Light</option>
          <option value="dark">Dark</option>
          <option value="system">System</option>
        </select>
      </label>

      <div class="actions">
        <button type="button" @click="openDashboard">Open dashboard</button>
        <button type="button" @click="close">Close</button>
        <button type="submit" :disabled="saving">{{ saving ? 'Saving…' : 'Save' }}</button>
      </div>

      <p v-if="message" class="msg">{{ message }}</p>
    </form>
  </main>
</template>

<style>
:root {
  font-family: system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif;
  font-size: 14px;
  color-scheme: light dark;
}
body {
  margin: 0;
}
main {
  padding: 20px 24px;
  max-width: 480px;
  margin: 0 auto;
}
h1 {
  font-size: 18px;
  margin: 0 0 16px;
}
form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}
label.row {
  flex-direction: row;
  align-items: center;
  gap: 8px;
}
input[type='url'],
input[type='password'],
select {
  padding: 6px 8px;
  border-radius: 4px;
  border: 1px solid #888;
  font: inherit;
  background: transparent;
  color: inherit;
}
.actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 8px;
}
button {
  padding: 6px 14px;
  border-radius: 4px;
  border: 1px solid #888;
  background: transparent;
  cursor: pointer;
  color: inherit;
  font: inherit;
}
button[type='submit'] {
  background: #2a7;
  color: white;
  border-color: #2a7;
}
.msg {
  font-size: 12px;
  color: #888;
  margin: 4px 0 0;
}
[data-theme='dark'] {
  background: #1e1e1e;
  color: #ddd;
}
[data-theme='light'] {
  background: #fafafa;
  color: #222;
}
</style>
