use std::vec::Vec;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub temperature: i32,
    pub api_token: String,
    pub api_group: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub wechat_nick: String,
    pub model: ModelConfig,
    pub hot_key: String,
}

#[derive(Serialize, Debug)]
pub struct ApiRequest<'a, T> {
    pub model: String,
    pub temperature: f32,
    pub messages: &'a Vec<T>,
    pub tokens_to_generate: i32,
    pub reply_constraints: ReplyConstraints<'a>,
    pub bot_setting: Vec<&'a HashMap<&'a str, &'a str>>,
}

#[derive(Serialize, Debug)]
pub struct ReplyConstraints<'a> {
    pub sender_type: &'a str,
    pub sender_name: &'a str,
}


impl<'a> ReplyConstraints<'a> {
    pub fn new_minimax(bot_name: &'a str) -> Self {
        ReplyConstraints {
            sender_type: "BOT",
            sender_name: bot_name
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ApiResponseBase {
    pub status_code: i32,
    pub status_msg: String
}

#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    pub reply: String,
    pub base_resp: ApiResponseBase
}
