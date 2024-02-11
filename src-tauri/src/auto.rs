use std::cmp::max;
use serde::Serialize;
use uiautomation::UIElement;
use uiautomation::actions::Window;
use uiautomation::variants::Variant;
use uiautomation::core::UIAutomation;
use uiautomation::types::{UIProperty, TreeScope};
use clipboard::{ClipboardContext, ClipboardProvider};
use uiautomation::controls::{ControlType, WindowControl};

#[derive(Debug)]
#[derive(Serialize)]
pub struct WechatHistory {
    pub text: String,
    pub sender_name: String,
    pub sender_type: String
}

#[derive(Debug)]
pub struct UiAutoSession {
    automation: UIAutomation,
    black_msg_list: Vec<&'static str>,
}

impl UiAutoSession {
    pub fn new() -> Self {
        // 初始化UiAutomation对象，以及可复用的条件
        let automation = UIAutomation::new().unwrap();    
        Self {
            automation: automation,
            black_msg_list: vec!["[图片]", "[动画表情]", "[音乐]", "[视频]", "[小程序]"]
        }
    }

    fn find_wechat_wnd(&self) -> Result<UIElement, String> {
        let root = self.automation.get_root_element().unwrap();
        let wechat = self.automation.create_matcher().from(
            root).classname("WeChatMainWndForPC").depth(3).timeout(0).find_first();
        if wechat.is_err() {
            return Err(String::from("未定位到微信窗口"));
        }

        // 条件变量，用于查找窗口
        let wechat = wechat.unwrap();
        let wechat_ctrl: Result<WindowControl, _> = wechat.clone().try_into();
        if wechat_ctrl.is_err() || wechat_ctrl.unwrap().is_minimized().is_ok_and(|cond| {cond}) {
            Err(String::from("未定位到微信窗口"))
        } else {
            Ok(wechat)
        }
    }

    pub fn wechat_content(&self) -> Result<Vec<WechatHistory>, String> {
        let wechat = self.find_wechat_wnd();
        if wechat.is_err() { return Err(wechat.unwrap_err()); }
        
        let wechat = wechat.unwrap();
        let button_cond = self.automation.create_property_condition(
            UIProperty::ControlType, Variant::from(0xC350), None).unwrap();
        let list_item_cond = self.automation.create_property_condition(
        UIProperty::ControlType, Variant::from(0xC357), None).unwrap();

        let walker = self.automation.create_tree_walker().unwrap();
        let chat_list = self.automation.create_matcher()
        .from(wechat).name("消息").control_type(ControlType::List)
        .depth(13).timeout(0).find_first();

        if chat_list.is_err() {
            Err(String::from("请先打开一个聊天页面"))
        } else if let Ok(msg_list) = chat_list.unwrap()
        .find_all(TreeScope::Children, &list_item_cond) {
            let real_text_cond = self.automation.create_property_condition(
                UIProperty::ControlType, Variant::from(0xC364), None).unwrap();
            let msg_list_collect = msg_list.iter().skip(max(0, (
                msg_list.len() as i32) - 20) as usize).map(|msg| {
                let mut str_sender: String = String::new();
                let mut str_content: String = String::new();
                let msg_name = msg.get_name();
                let msg_real_text = msg.find_all(
                    TreeScope::Descendants, &real_text_cond);
                let msg_sender = walker.get_last_child(&msg).and_then(
                |node| {node.find_first(TreeScope::Children, &button_cond)});
                if msg_name.is_ok() && msg_sender.is_ok() {
                    str_content = msg_name.unwrap();
                    str_sender = msg_sender.unwrap().get_name().unwrap();
                    if msg_real_text.is_ok() {
                        let real_text_vec = msg_real_text.unwrap();
                        let msg_real_text = real_text_vec.iter().find(|item| {
                            !item.get_name().unwrap().is_empty() && walker.get_parent(&item).is_ok_and(
                            |parent| {walker.get_next_sibling(&parent).is_err()})
                        });
                        if msg_real_text.is_some() {
                            str_content = msg_real_text.unwrap().get_name().unwrap();
                        }
                    }
                }
                WechatHistory {
                    text: str_content,
                    sender_name: str_sender,
                    sender_type: String::from("USER")
                }
            }).filter(|chat| {!chat.text.is_empty() && !self
            .black_msg_list.contains(&chat.text.as_str())}).collect::<Vec<WechatHistory>>();

            if msg_list_collect.is_empty() {
                Err(String::from("未找到可分析的聊天记录"))
            } else {
                Ok(msg_list_collect)
            }
        } else {
            Err(String::from("未找到可分析的聊天记录"))
        }
    }

    pub fn wechat_send(&self, text: String, direct_send: bool) -> Result<(), String> {
        let wechat = self.find_wechat_wnd();
        if wechat.is_err() { return Err(wechat.unwrap_err()); }
        
        let wechat = wechat.unwrap();
        let walker = self.automation.create_tree_walker().unwrap();
        let edit_cond = self.automation.create_property_condition(
            UIProperty::ControlType, Variant::from(0xC354), None).unwrap();
        let send_button = self.automation.create_matcher().from(wechat)
        .control_type(ControlType::Button).timeout(0).name(String::from("发送(S)")).depth(16).find_first();
        if send_button.is_err() {
            return Err(String::from("无法定位到发送按钮，请稍后重试"));
        }
        let send_button = send_button.unwrap();
        if let Ok(edit_box) = walker.get_parent(&send_button)
        .and_then(|node| walker.get_parent(&node))
        .and_then(|node| walker.get_parent(&node))
        .and_then(|parent| parent.find_first(TreeScope::Descendants, &edit_cond)) 
        {
            if !edit_box.has_keyboard_focus().unwrap() {
                if edit_box.click().is_err() {
                    return Err(String::from("点击消息窗口失败，请检查微信窗口是否可见"));
                }
            }
            let mut clip = ClipboardContext::new().unwrap();
            let clip_backup = if let Ok(content) = clip.get_contents() 
            { content } else { String::new() };

            if clip.set_contents(text).is_err() {
                return Err(String::from("无法复制消息，请稍后重试"));
            }
            if let Ok(()) = edit_box.send_keys("{ctrl}V", 20) {
                let _ = clip.set_contents(clip_backup).is_ok();    
                if !direct_send {
                    Ok(())
                } else {
                    let _ = send_button.click().is_ok();
                    Ok(())
                }
            } else {
                return Err(String::from("无法复制消息，请稍后重试"));
            }
        } else {
            Err(String::from("无法定位到消息框，请稍后重试"))
        }

    }
}