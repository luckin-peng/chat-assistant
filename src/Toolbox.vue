<script setup>
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { getCurrent } from '@tauri-apps/api/window';
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from '@tauri-apps/api/event';
import { onMounted, ref } from 'vue';

var isBusy = false;
const errMessage = ref('');
const messageList = ref([]);
const ctrlKeyDown = ref(false);
const displayStatus = ref('loading');

function hideWindow() {
  getCurrent().hide();
  displayStatus.value = 'closed';
}

function refreshReply() {
  displayStatus.value = 'loading';
  invoke('get_reply_content').then(resp => {
    if (displayStatus.value === 'loading') {
      messageList.value = resp;
      displayStatus.value = 'finish';
    }
  }).catch(errMsg => {
    if (displayStatus.value === 'loading') {
      displayStatus.value = 'error';
      errMessage.value = errMsg;
    }
  });
}

function submitWechat(caller) {
  if (isBusy) return;
  isBusy = true;
  const text = caller.target.innerText;
  const ctrlPressed = ctrlKeyDown.value;
  invoke('submit_wechat', {"text": text, "ctrlPressed": ctrlPressed})
  .then(_ => {
    hideWindow();
  }).finally(() => {
    isBusy = false;
  });
}

onMounted(async () => {
  listen('show', (_) => {
    refreshReply();
  });

  window.onkeydown = (e) => {
    if (e.key === 'Escape') {
      hideWindow();
    } else if (e.key === 'Control') {
      ctrlKeyDown.value = true;
    }
  }

  window.onkeyup = (e) => {
    if (e.key === 'Control') {
      ctrlKeyDown.value = false;
    }
  }
});
</script>

<template>
  <div class="container" v-if="displayStatus === 'finish'">
    <div class="chatContainer">
      <div class="chatMsg" v-for="chatMsg in messageList" @click="submitWechat">{{ chatMsg }}</div>
    </div>
    <div class="ops">
      <div class="op" v-if="!ctrlKeyDown" @click="refreshReply">✒️ 换一批</div>
      <div class="op disabled" v-if="ctrlKeyDown">✈️ 点击消息直接发送</div>
      <div class="op" @click="hideWindow">⭕ 取消 (Esc)</div>
    </div>
  </div>

  <div class="container" v-if="displayStatus === 'loading'">
    <div class="loadingio-spinner-pulse">
      <div class="loadingio-spinner">
        <div></div><div></div><div>
        </div><div></div><div></div>
    </div></div>
    <div class="exitHint">如需取消生成，请按Esc键</div>
  </div>

  <div class="container" v-if="displayStatus === 'error'">
    <div class="errorMsg">{{ errMessage }}</div>
    <div class="exitHint">可以按下Esc键退出当前页面</div>
  </div>
</template>