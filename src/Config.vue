<script setup>
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { message, confirm } from '@tauri-apps/api/dialog';
import { getCurrent } from '@tauri-apps/api/window';
import { invoke } from "@tauri-apps/api/tauri";
import { onMounted, ref } from 'vue';

const modelTemperature = ref('');
const modelApiGroup = ref('');
const modelApiToken = ref('');
const modelProvider = ref('');
const initMode = ref(false);
const wechatNick = ref('');
const modelName = ref('');
const hotKey = ref('');

function updateConfig() {
    invoke('save_config', {"config": {
        "wechat_nick": wechatNick.value,
        "hot_key": hotKey.value,
        "model": {
            "temperature": parseInt(modelTemperature.value),
            "api_group": modelApiGroup.value,
            "api_token": modelApiToken.value,
            "provider": modelProvider.value,
            "name": modelName.value
        }
    }}).then(_ => {
        getCurrent().close();
    }).catch((msg) => {
        message(msg, {type: 'warning', title: '保存失败'});
    });
}

function resetAndExit() {
    confirm('确定要删除所有配置并退出吗？该操作不可逆。\n重置完成后，你可以重新初始化，或直接删除程序。', 
    {title: '删除配置并退出'}).then((res) => {
        if (res) {
            invoke('reset_and_exit').catch(msg => {
                message(msg, {type: 'warning', title: '重置失败'});
            });
        }
    });
}

onMounted(() => {
    invoke('load_config').then(config => {
        modelTemperature.value = config.model.temperature;
        modelApiGroup.value = config.model.api_group;
        modelApiToken.value = config.model.api_token;
        modelProvider.value = config.model.provider;
        wechatNick.value = config.wechat_nick;
        modelName.value = config.model.name;
        hotKey.value = config.hot_key;
    }).catch(() => {
        initMode.value = true;
    });
})
</script>

<template>
    <div class="initTips" v-if="initMode">没有找到配置文件，当前是初始化模式，请填写所有项目，然后保存。</div>
    <h3>热键设置</h3>
    <div class="flexItem"><div class="title">Ctrl + Alt + </div><input class="short" type="text" maxlength="1" v-model="hotKey"></div>
    <h3>微信设置</h3>
    <div class="item"><div class="title">微信昵称：</div><input type="text" placeholder="填写错误可能会影响生成结果" v-model="wechatNick"></div>
    <h3>模型设置</h3>
    <div class="item"><div class="title">模型提供商：</div><input type="text" placeholder="当前版本仅支持MiniMax" v-model="modelProvider"></div>
    <div class="item"><div class="title">模型名称：</div><input type="text" placeholder="支持abab6-chat、abab5.5-chat" v-model="modelName"></div>
    <div class="item"><div class="title">API Group：</div><input type="text" placeholder="填写Group ID" v-model="modelApiGroup"></div>
    <div class="item"><div class="title">API Key：</div><input type="text" placeholder="填写API Key" v-model="modelApiToken">
    <div class="tips">申请API可前往：<a target="_blank" href="https://api.minimax.chat/">https://api.minimax.chat/</a></div>
    </div>
    <div class="item"><div class="title">随机度：</div><input type="number" min="1" max="100" step="1" placeholder="越大代表产生的结果越随机" v-model="modelTemperature"></div>
    <h3 v-if="!initMode">重置设置</h3>
    <div class="item" v-if="!initMode"><div class="reset" @click="resetAndExit">删除配置并退出</div></div>
    <div class="ops"><div class="op" @click="updateConfig">保 存</div><div class="op" @click="getCurrent().close()">取 消</div></div>
</template>

<style>
    :root {
        margin: 0;
        padding: 0;
        width: 100%;
        height: 100%;
        overflow: hidden;
        color: #303030;
        user-select: none;
    }

    body {
        margin: 0;
        height: 100%;
        overflow-y: auto;
        padding-bottom: 1.5rem;
        box-sizing: border-box;
        padding: 0.5rem 0.5rem 1rem 0.5rem;
    }

    .tips {
        font-size: 13px;
    }

    a {
        text-decoration: none;
    }

    a:visited {
        color: blue;
    }

    .flexItem {
        width: 100%;
        display: flex;
        font-size: 16px;
        padding: 0.25rem 0;
        align-items: center;
        justify-content: start;
        box-sizing: border-box;
    }

    .flexItem .short {
        width: 2rem;
        font-size: 16px;
        text-align: center;
        margin-left: 0.5rem;
    }

    .item {
        width: 100%;
        font-size: 16px;
        padding: 0.25rem 0;
        display: inline-block;
        box-sizing: border-box;
    }

    .tips {
        margin-top: 0.25rem;
    }

    h3 {
        padding: 0;
        margin: 0.5rem 0 0 0;
    }

    h3:nth-child(1) {
        margin: 0;
    }

    .title {
        padding: 0.25rem 0;
    }

    input {
        width: 100%;
        outline: none;
        font-size: 14px;
        padding: 6px 6px;
        border-radius: 6px;
        display: inline-block;
        box-sizing: border-box;
        border: solid 1px #A0A0A0;
    }

    input:focus {
        border: solid 1px #07C160;
    }

    .ops {
        display: flex;
        margin-top: 1rem;
        justify-content: end;
    }

    .ops .op {
        cursor: pointer;
        color: #FFFFFF;
        margin-left: 1rem;
        border-radius: 6px;
        padding: 0.5rem 2.5rem;
        box-sizing: border-box;
        background-color: #A0A0A0;
    }

    .ops .op:hover {
        background-color: #07C160;
    }

    .initTips {
        font-size: 14px;
        color: #FFFFFF;
        padding: 0.25rem 0.25rem;
        background-color: #07C160;
    }

    .reset {
        width: auto;
        color: coral;
        cursor: pointer;
        margin-top: 0.5rem;
        border-radius: 6px;
        display: inline-block;
        padding: 0.5rem 1.5rem;
        box-sizing: border-box;
        border: solid 1px coral;
    }

    .reset:hover {
        color: #FFFFFF;
        background-color: coral;
    }
    
</style>
