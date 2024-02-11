<script setup>
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { getCurrent } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { onMounted, ref } from 'vue';

const messageContent = ref('');

onMounted(() => {
    listen('message', (event) => {
      getCurrent().show();
      messageContent.value = event.payload;
      setTimeout(() => { getCurrent().hide() }, 2500);
    });
})
</script>

<template>
  <div class="toastMessage">{{ messageContent }}</div>
</template>

<style>
  :root {
    height: 100%;
    overflow: hidden;
  }

  body, #app {
    height: 100%;
    overflow: hidden;
  }

  .toastMessage {
    width: 100%;
    display: flex;
    padding: 1rem 0;
    color: #FFFFFF;
    overflow: hidden;
    font-size: 1.25em;
    user-select: none;
    align-items: center;
    border-radius: 0.5rem;
    box-sizing: border-box;
    justify-content: center;
    background-color: rgba(0, 0, 0, 0.6);
  }
</style>
