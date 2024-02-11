#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// https://tauri.app/v1/guides/features/command

mod auto;
mod conf;
use serde_json;
use std::path::Path;
use tokio::runtime::Runtime;
use std::collections::HashMap;
use windows::core::{w, PCWSTR};
use std::sync::{OnceLock, Mutex};
use window_vibrancy::apply_acrylic;
use auto::{UiAutoSession, WechatHistory};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::Foundation::{GetLastError, HWND, RECT};
use conf::{ApiRequest, AppConfig, ReplyConstraints, ApiResponse};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{self, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible};
use tauri::{App, AppHandle, CustomMenuItem, GlobalShortcutManager, Manager, PhysicalPosition, PhysicalSize, Position, Size, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, Window, WindowBuilder, WindowEvent};

static CONFIG: OnceLock<Mutex<AppConfig>> = OnceLock::new();
static SESSION: OnceLock<Mutex<ToolSession>> = OnceLock::new();

unsafe extern "system" fn handle_win_event(_event_hook: HWINEVENTHOOK, event_type: u32, 
wechat_hwnd: HWND, id_object: i32, id_child: i32, _thread_id: u32, _timestamp: u32) {
    if id_object != WindowsAndMessaging::OBJID_WINDOW.0 || id_child != 0 
    || (event_type != WindowsAndMessaging::EVENT_OBJECT_HIDE 
        && event_type != WindowsAndMessaging::EVENT_OBJECT_DESTROY 
        && event_type != WindowsAndMessaging::EVENT_SYSTEM_MOVESIZEEND) {
        return;
    } else if event_type == WindowsAndMessaging::EVENT_OBJECT_DESTROY || 
    (event_type == WindowsAndMessaging::EVENT_OBJECT_HIDE && !IsWindowVisible(wechat_hwnd).as_bool()) {
        if let Ok(mut session) = SESSION.get().unwrap().lock() {
            session.detach_hook();    
            let _ = session.window.hide().is_ok();
        }
    } else if event_type == WindowsAndMessaging::EVENT_SYSTEM_MOVESIZEEND {
        let mut wechat_rect:RECT = Default::default();
        if GetWindowRect(wechat_hwnd, &mut wechat_rect).is_err() {
            return;
        }
        if let Ok(session) = SESSION.get().unwrap().lock() {
            let _ = session.window.set_position(Position::Physical(PhysicalPosition{
                x: wechat_rect.left as i32, y: wechat_rect.bottom as i32})).is_err();
            let _ = session.window.set_size(PhysicalSize{
                width: (wechat_rect.right - wechat_rect.left) as u32, 
                height: (204.0 * session.sys_dpi) as u32}).is_err();    
        }
    }
}

struct ToolSession {
    sys_dpi: f64,
    window: Window,
    event_hook: HWINEVENTHOOK,
}

impl ToolSession {
    fn new(app_handle: &AppHandle) -> ToolSession {
        let rt = Runtime::new().unwrap();
        let wnd = rt.block_on(init_tool_wnd(app_handle));
        let dpi: f64 = wnd.scale_factor().or::<f64>(Ok(1.0f64)).unwrap();
        ToolSession {
            window: wnd,
            sys_dpi: dpi,
            event_hook: HWINEVENTHOOK::default()
        }
    }

    fn attach_wechat(&mut self) -> Result<(), &str> {
        unsafe {
            if !self.event_hook.is_invalid() {
                UnhookWinEvent(self.event_hook);
            }
            let fore_hwnd = GetForegroundWindow().0;
            let wechat_hwnd = WindowsAndMessaging::FindWindowW(
                w!("WeChatMainWndForPC"), PCWSTR::null());
            
            if wechat_hwnd.0 == 0 || wechat_hwnd.0 != fore_hwnd {
                return Err("请确保微信窗口处于激活状态");
            }
            let mut wechat_rect:RECT = Default::default();
            if GetWindowRect(wechat_hwnd, &mut wechat_rect).is_err() 
            || wechat_rect.right <= 0 || wechat_rect.bottom <= 0 {
                return Err("未定位到微信窗口");
            }

            // 设置窗口移动消息监听钩子
            let mut wechat_process = 0u32;
            let thread = GetWindowThreadProcessId(wechat_hwnd, Some(&mut wechat_process));
            self.event_hook = SetWinEventHook(WindowsAndMessaging::EVENT_SYSTEM_MOVESIZEEND, 
            WindowsAndMessaging::EVENT_OBJECT_HIDE, None, Some(handle_win_event), 
            wechat_process, thread, WindowsAndMessaging::WINEVENT_OUTOFCONTEXT);
            
            let _ = self.window.set_position(Position::Physical(PhysicalPosition{
                x: wechat_rect.left as i32, y: wechat_rect.bottom as i32})).is_err();
            let _ = self.window.set_size(Size::Physical(PhysicalSize{
                width: (wechat_rect.right - wechat_rect.left) as u32, 
                height: (204.0 * self.sys_dpi) as u32})).is_err();
            if self.window.emit("show", "").is_err() {
                Err("激活窗口失败，请稍后重试")
            } else if self.window.show().is_err() || self.window.set_focus().is_err() {
                Err("展示窗口失败，请稍后重试")
            } else {
                Ok(())
            }
        }
    }

    fn detach_hook(&mut self) {
        unsafe {
            if !self.event_hook.is_invalid() {
                UnhookWinEvent(self.event_hook);
                self.event_hook = HWINEVENTHOOK::default();
            }
        }
    }
}

async fn get_ai_reply(config: AppConfig, chat_hist: &mut Vec<WechatHistory>) -> Result<String, String> {
    let request_url = match config.model.provider.to_uppercase().as_str() {
        "MINIMAX" => format!("https://api.minimax.chat/v1/text/chatcompletion_pro?GroupId={}", &config.model.api_group),
        _ => { return Err(format!("不支持的模型提供商：{}", config.model.provider)); }
    };
    let model_name = config.model.name.to_lowercase();
    let max_token = match model_name.as_str() {
        "abab6-chat" => 2048,
        "abab5.5-chat" => 2048,
        _ => { return Err(format!("不支持的模型：{}", config.model.name)); }
    };

    let command = format!("阅读{}和别人的对话记录，从{}的视角产出5条回复。", config.wechat_nick, config.wechat_nick);
    let mut bot_settings = HashMap::new();
    bot_settings.insert("content", command.as_str());
    bot_settings.insert("bot_name", "智能回复助手");

    chat_hist.push(WechatHistory{
        sender_type: String::from("USER"),
        sender_name: config.wechat_nick.clone(),
        text: format!("以上是我和其他人的对话记录，请结合上述记录，产出5条回复建议。\n{}", 
        "要求：给出5条不同的回复，有些回复简短一些，有些回复更长。回复不要带序号，不要输出回复建议之外的任何内容，不同的回复之间需要空两行。")
    });
    let temperature = config.model.temperature as f32 / 100.0;
    let reply_constraints = ReplyConstraints::new_minimax("智能回复助手");
    let req_body = ApiRequest::<WechatHistory> {
        model: model_name,
        messages: &chat_hist,
        temperature: temperature,
        tokens_to_generate: max_token,
        bot_setting: vec![&bot_settings],
        reply_constraints: reply_constraints
    };                    

    let resp_res = reqwest::Client::new().post(&request_url)
    .json(&req_body).header("Authorization", format!("Bearer {}", config.model.api_token))
    .header("Content-Type", "application/json").send().await;
    if resp_res.is_ok() {
        let resp = resp_res.unwrap();
        let resp_json: Result<ApiResponse, _> = resp.json().await;
        if resp_json.as_ref().is_ok_and(|js: &ApiResponse| {js.base_resp.status_code == 0}) {
            Ok(resp_json.unwrap().reply)
        } else if resp_json.is_ok() {
            let err_msg = resp_json.unwrap().base_resp.status_msg;
            println!("获取回复失败，err_msg：{}", err_msg);
            Err(String::from("获取回复失败，请稍后重试"))
        } else {
            Err(String::from("解析回复内容失败，请稍后重试"))
        }
    } else {
        let err_msg = resp_res.unwrap_err().to_string();
        println!("获取回复失败，err_msg：{}", err_msg);
        Err(String::from("网络请求失败，请稍后重试"))
    }
}

async fn init_tool_wnd(app_handle: &AppHandle) -> Window {
    let window = tauri::WindowBuilder::new(app_handle, "toolWnd", 
    tauri::WindowUrl::App("toolbox.html".into())).visible(false).skip_taskbar(true)
    .transparent(true).decorations(false).resizable(false).build().unwrap();
    let _ = apply_acrylic(&window, Some((128, 128, 128, 1))).is_err();
    window.on_window_event(|event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
        }
        _ =>  {}
    });
    window
}

fn message_toast(app_handle: &AppHandle, message: &str) {
    app_handle.emit_to("toastWnd", "message",
    String::from(message)).unwrap();
}

fn shortcut_actived(app_handle: &AppHandle) {
    if let Ok(mut wechat) = SESSION.get().unwrap().lock() {
        if let Err(msg) = wechat.attach_wechat() {
            message_toast(app_handle, msg);
        }
    } else {
        message_toast(app_handle, "启动失败，请稍后重试");
    }    
}

#[tauri::command]
async fn submit_wechat(text: String, ctrl_pressed: bool, app_handle: tauri::AppHandle) -> Result<(), ()> {
    let session = UiAutoSession::new();
    if let Err(message) = session.wechat_send(text, ctrl_pressed) {
        message_toast(&app_handle, message.as_str());
        Err(())
    } else {
        Ok(())
    }
}

#[tauri::command]
async fn get_reply_content() -> Result<Vec<String>, String> {
    let app_config: AppConfig;
    let mut chat_messages: Vec<WechatHistory>;
    {
        let uia = auto::UiAutoSession::new();
        let wechat_resp = uia.wechat_content();
        if wechat_resp.is_ok() {
            chat_messages = wechat_resp.unwrap();
        } else {
            return Err(wechat_resp.unwrap_err());
        }
        if let Ok(config) = CONFIG.get().unwrap().try_lock() {
            app_config = config.clone();
        } else {
            return Err(String::from("请求过于频繁，请稍后重试"));
        }
    }
    
    if chat_messages.is_empty() {
        Err(String::from("未找到可供分析的聊天记录，无法产出建议"))
    } else {
        let resp = get_ai_reply(
            app_config, &mut chat_messages).await;
        if resp.is_ok() {
            let message = resp.unwrap();
            let result: Vec<String> = message.split("\n").into_iter().filter(
            |line| !line.is_empty()).map(|s| String::from(
                s.trim().trim_end_matches('。'))).collect();
            if result.is_empty() {
                Err(String::from("未产出有价值的建议，请稍后重试"))
            } else if result.len() == 1 {
                let result: Vec<String> = message.split(' ').into_iter().filter(
                    |line| !line.is_empty()).map(|s| String::from(
                        s.trim().trim_end_matches('。'))).collect();
                Ok(result)
            } else {
                Ok(result)
            }
        } else {
            Err(resp.unwrap_err())
        }
    }
}

#[tauri::command]
fn reset_and_exit(app_handle: AppHandle) -> Result<(), String> {
    let sys_path = std::env::var_os("LOCALAPPDATA").unwrap();
    let app_config_root = Path::new(sys_path.to_str().unwrap());
    let app_config_root = app_config_root.join("ChatAssistant");
    if std::fs::remove_dir_all(app_config_root).is_err() {
        Err(String::from("删除配置文件失败，请检查权限问题"))
    } else {
        let _ = app_handle.tray_handle().destroy().is_ok();
        std::process::exit(0);
    }
}

#[tauri::command]
fn load_config() -> Result<AppConfig, ()> {
    if let Ok(_) = std::panic::catch_unwind(|| {
        CONFIG.get_or_init(|| {
            let sys_path = std::env::var_os("LOCALAPPDATA").unwrap();
            let app_config_root = Path::new(sys_path.to_str().unwrap());
            let app_config_root = app_config_root.join("ChatAssistant").join("config.json");
            let config_str = std::fs::read_to_string(app_config_root).unwrap();
            let config: Result<AppConfig, _> = serde_json::from_str(&config_str);
            Mutex::new(config.unwrap())
        });    
    }) {
        Ok(CONFIG.get().unwrap().lock().unwrap().clone())
    } else {
        Err(())
    }
}

#[tauri::command]
fn save_config(config: AppConfig, app_handle: tauri::AppHandle) -> Result<(), String> {
    if config.hot_key.len() != 1 || config.wechat_nick.is_empty() {
        Err(String::from("配置不完整"))
    } else if config.model.temperature <= 0 || config.model.temperature > 100 {
        Err(String::from("随机度应介于1-100之间"))
    } else if config.model.provider.is_empty() || config.model.name.is_empty() {
        Err(String::from("请填写模型相关配置"))
    } else if config.model.api_group.is_empty() || config.model.api_token.is_empty() {
        Err(String::from("请填写模型API相关配置"))
    } else if CONFIG.get().is_none() {
        let config_json = serde_json::to_string(&config);
        let acce = format!("CommandOrControl+Alt+{}", config.hot_key);
        CONFIG.get_or_init(|| { Mutex::new(config) });
        if app_handle.global_shortcut_manager().register(acce.as_str(), move || {
            shortcut_actived(&app_handle);
        }).is_err() {
            return Err(String::from("注册快捷键失败"));
        }
        let sys_path = std::env::var_os("LOCALAPPDATA").unwrap();
        let app_data_path = sys_path.to_str().unwrap();
        let app_config_root = Path::new(app_data_path);
        let app_config_root = app_config_root.join("ChatAssistant");
        if !app_config_root.exists() && std::fs::create_dir(&app_config_root).is_err() {
            Err(String::from("写入配置失败，请检查权限问题"))
        } else if std::fs::write(app_config_root.join("config.json"), config_json.unwrap()).is_err() {
            Err(String::from("保存配置失败，请检查权限问题"))
        } else {
            Ok(())
        }
    } else if let Ok(mut old_config) = CONFIG.get().unwrap().try_lock() {
        let old_hot_key = old_config.hot_key.clone();
        let config_json = serde_json::to_string(&config);
        old_config.model = config.model;
        old_config.hot_key = config.hot_key.clone();
        old_config.wechat_nick = config.wechat_nick;
        if old_hot_key != config.hot_key {
            let old_acce = format!("CommandOrControl+Alt+{}", old_hot_key);
            let new_acce = format!("CommandOrControl+Alt+{}", config.hot_key);
            let _ = app_handle.global_shortcut_manager().unregister(&old_acce).is_ok();
            if app_handle.global_shortcut_manager().register(new_acce.as_str(), move || {
                shortcut_actived(&app_handle);
            }).is_err() {
                return Err(String::from("注册快捷键失败"));
            }
        }
        let sys_path = std::env::var_os("LOCALAPPDATA").unwrap();
        let app_data_path = sys_path.to_str().unwrap();
        let app_config_root = Path::new(app_data_path);
        let app_config_root = app_config_root.join("ChatAssistant");
        if !app_config_root.exists() && std::fs::create_dir(&app_config_root).is_err() {
            Err(String::from("写入配置失败，请检查权限问题"))
        } else if std::fs::write(app_config_root.join("config.json"), config_json.unwrap()).is_err() {
            Err(String::from("保存配置失败，请检查权限问题"))
        } else {
            Ok(())
        }
    } else {
        Err(String::from("保存配置失败，系统繁忙，请稍后重试"))
    }
}

fn main() {
    unsafe {
        let mutex =  CreateMutexW(None,
             true, w!("ChatAssistant"));
        if mutex.is_err() || GetLastError().is_err() {
            println!("已经运行一个实例，退出");
            return;
        }
    }
    let config = load_config();
    let menu_config = CustomMenuItem::new("config".to_string(), "打开设置");
    let menu_about = CustomMenuItem::new("about".to_string(), "关于瓜皮助手");
    let menu_exit = CustomMenuItem::new("exit".to_string(), "退出");
    let tray_menu = SystemTrayMenu::new().add_item(menu_config).add_item(menu_about)
        .add_native_item(SystemTrayMenuItem::Separator).add_item(menu_exit);
    let system_tray = SystemTray::new().with_menu(tray_menu).with_tooltip("瓜皮助手");

    let app = tauri::Builder::default()
    .system_tray(system_tray).setup(move|app: &mut App| {
        let handle = app.handle();
        SESSION.get_or_init(|| Mutex::new(ToolSession::new(&handle)));
        if config.is_ok() {
            let accelerator = format!("CommandOrControl+Alt+{}", config.unwrap().hot_key);
            if app.global_shortcut_manager().is_registered(accelerator.as_str())
            .is_ok_and(|res| !res) && app.global_shortcut_manager().register(accelerator.as_str(), 
            move|| { shortcut_actived(&handle) }).is_err() {
                tauri::api::dialog::message(None::<&tauri::Window>, "快捷键冲突", 
                "注册快捷键失败，将无法唤醒瓜皮助手。请检查快捷键是否冲突，然后重启程序，或者进入设置修改快捷键。");
            }    
        } else if let Ok(config_wnd) = WindowBuilder::new(&app.app_handle(), "configWnd", 
        tauri::WindowUrl::App("./config.html".into())).center().inner_size(540.0, 410.0)
        .title("初始化 - 瓜皮助手").resizable(false).minimizable(true).build() {
            let _ = config_wnd.show().is_ok();
        } else {
            tauri::api::dialog::message(None::<&tauri::Window>, "初始化失败", 
            "初始化配置窗口失败，请检查权限问题");
            std::process::exit(1);
        }
        Ok(())
    }).on_system_tray_event(|app, event| match event {
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "config" => {
                    if let Ok(config_wnd) = WindowBuilder::new(&app.app_handle(), "configWnd", 
                    tauri::WindowUrl::App("./config.html".into())).center().inner_size(540.0, 410.0)
                    .title("设置 - 瓜皮助手").resizable(false).minimizable(true).build() {
                        let _ = config_wnd.show().is_ok();
                    }
                }
                "about" => {
                    if let Ok(about_wnd) = WindowBuilder::new(&app.app_handle(), "aboutWnd", 
                    tauri::WindowUrl::App("./about.html".into())).center().inner_size(400.0, 200.0)
                    .title("关于 - 瓜皮助手").resizable(false).minimizable(false).always_on_top(true).build() {
                        let _ = about_wnd.show().is_ok();
                    }
                }
                "exit" => {
                    let _ = app.tray_handle().destroy().is_ok();
                    std::process::exit(0); 
                }
                _ => {}
            }
        }    
        _ => ()
    }).invoke_handler(tauri::generate_handler![get_reply_content, submit_wechat, load_config, save_config, reset_and_exit])
    .build(tauri::generate_context!()).expect("启动APP失败，请重试！");

    app.run(|_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
          api.prevent_exit();
        }
        _ => {}
    });
}
