<script setup lang="ts">
import { reactive, watch } from 'vue';
import { ArrowRight, Globe2, Server, Save, UserRound, SlidersHorizontal } from 'lucide-vue-next';
import { useWorkspaceStore } from '@/stores/workspace';
import { isTauriRuntime } from '@/services/tauri';

const store = useWorkspaceStore();
const form = reactive({ ...store.state.settings });
watch(
  () => store.state.settings,
  (value) => Object.assign(form, value),
);
function submit(open: boolean) {
  try {
    const login = new URL(form.loginUrl);
    const api = new URL(form.apiBaseUrl);
    if (
      ![login, api].every(
        (url) => ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password,
      )
    )
      throw new Error('请输入有效的 HTTP(S) 地址');
    if (new URL(form.landingPath, login).origin !== login.origin)
      throw new Error('登录后地址必须与登录页同源');
    void store.save({ ...form }, open);
  } catch (reason) {
    store.report(reason);
  }
}
</script>

<template>
  <section class="settings-page">
    <div class="settings-heading">
      <div class="section-icon"><SlidersHorizontal :size="24" /></div>
      <p class="eyebrow">工作区设置</p>
      <h2>连接系统，登录主账号</h2>
    </div>
    <form class="settings-form" @submit.prevent="submit(true)">
      <div class="field">
        <label for="login-url">系统登录地址</label>
        <div class="input-icon">
          <Globe2 :size="18" /><input
            id="login-url"
            v-model.trim="form.loginUrl"
            type="url"
            required
            autocomplete="url"
            placeholder="https://your-system.example.com"
            :disabled="store.busy"
          />
        </div>
      </div>
      <div class="field">
        <label for="api-url">系统接口地址</label>
        <div class="input-icon">
          <Server :size="18" /><input
            id="api-url"
            v-model.trim="form.apiBaseUrl"
            type="url"
            required
            :disabled="store.busy"
          />
        </div>
      </div>
      <div class="field">
        <label for="landing-path">子账号登录后地址</label>
        <input id="landing-path" v-model.trim="form.landingPath" required :disabled="store.busy" />
      </div>
      <label class="checkbox-row"
        ><input
          v-model="form.remember"
          type="checkbox"
          :disabled="store.busy"
        />记住地址、主账号并保持登录状态</label
      >
      <div v-if="form.username && form.remember" class="remembered-account">
        <UserRound :size="18" /><span>上次登录</span><strong>{{ form.username }}</strong>
      </div>
      <div class="form-actions">
        <button
          class="button secondary"
          type="button"
          :disabled="store.busy"
          @click="submit(false)"
        >
          <Save :size="16" />保存设置
        </button>
        <button class="button primary" type="submit" :disabled="store.busy">
          {{ store.busy ? '正在连接' : '打开主账号' }}<ArrowRight :size="17" />
        </button>
      </div>
    </form>
    <p v-if="!isTauriRuntime()" class="preview-note">浏览器预览 · 账号会话需要在桌面客户端中打开</p>
  </section>
</template>
