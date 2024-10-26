use crate::pp_calculator;
use crate::osu_api;

use irc::client::prelude::*;
use std::error::Error;
use futures::stream::StreamExt;
use std::time::{Instant, Duration};
use crate::commands::handle_command;
use regex::Regex;
use tokio::time::interval;
use self::pp_calculator::PPCalculator;
use self::osu_api::OsuApi;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use crate::events::handle_event;
use std::env;
use std::collections::HashMap;
use crate::osu_api::User;
use std::fs::File;
use std::io::{Write, Read};

pub struct MyBot {
    client: Client,
    pub player_list: Vec<String>,
    pub room_host_list: Vec<String>,
    pub beatmap_start_time: Option<Instant>,
    pub beatmap_end_time: Option<Instant>,
    pub approved_abort_list: Vec<String>,
    pub approved_start_list: Vec<String>,
    pub approved_skip_list: Vec<String>,
    pub approved_close_list: Vec<String>,
    pub room_host: String,
    pub room_id: Arc<TokioMutex<u32>>,
    pub room_name: String,
    pub room_password: String,
    pub beatmap_id: u32,
    pub beatmap_length: u64,
    pub beatmap_path: String,
    pub pp_calculator: PPCalculator,
    pub osu_api: OsuApi,
    pub player_info: HashMap<String, User>,
    pub beatmap_title_unicode: String,
    pub beatmap_artist_unicode: String,
    pub beatmap_difficulty_rating: f32,
    pub beatmap_info: String,
    pub beatmap_pp_info: String,
}

impl MyBot {
    pub async fn new(config: Config, client_id: String, client_secret: String) -> Result<Self, Box<dyn Error>> {
        let client = Client::from_config(config).await?;
        
        // 尝试读取上次保存的房间ID
        let last_room_id = Self::read_last_room_id().unwrap_or(0);
        
        let bot = MyBot {
            client,
            player_list: Vec::new(),
            room_host_list: Vec::new(),
            beatmap_start_time: None,
            beatmap_end_time: None,
            approved_abort_list: Vec::new(),
            approved_start_list: Vec::new(),
            approved_skip_list: Vec::new(),
            approved_close_list: Vec::new(),
            room_host: String::new(),
            room_id: Arc::new(TokioMutex::new(last_room_id)),
            room_name: env::var("ROOM_NAME").unwrap_or_else(|_| "".to_string()),
            room_password: env::var("ROOM_PASSWORD").unwrap_or_else(|_| "".to_string()),
            beatmap_id: 0,
            beatmap_length: 0,
            beatmap_path: String::new(),
            pp_calculator: PPCalculator::new(String::new()),
            osu_api: OsuApi::new(client_id, client_secret),
            player_info: HashMap::new(),
            beatmap_title_unicode: String::new(),
            beatmap_artist_unicode: String::new(),
            beatmap_difficulty_rating: 0.0,
            beatmap_info: String::new(),
            beatmap_pp_info: String::new(),
        };

        Ok(bot)
    }

    pub fn read_last_room_id() -> Result<u32, Box<dyn Error>> {
        let mut file = File::open("last_room_id.txt")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents.trim().parse()?)
    }

    pub async fn join_last_room(&self) -> Result<(), Box<dyn Error>> {
        let room_id = *self.room_id.lock().await;
        self.join_channel(&format!("#mp_{}", room_id)).await?;
        println!("Joined last room: #mp_{}", room_id);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.client.identify()?;

        let room_id = *self.room_id.lock().await;
        if room_id == 0 {
            // 如果没有上次的房间ID,创建新房间
            self.create_room().await?;
        } else {
            println!("Using existing room: #mp_{}", room_id);
            // 尝试加入上次的房间
            self.join_last_room().await?;
            self.get_mp_settings().await?;
        }

        let mut stream = self.client.stream()?;

        // Start the periodic task
        // let room_id = Arc::clone(&self.room_id);
        // tokio::spawn(async move {
        //     let mut interval = interval(Duration::from_secs(60));
        //     loop {
        //         interval.tick().await;
        //         let current_room_id = *room_id.lock().await;
        //         if let Err(e) = Self::check_room_status(current_room_id).await {
        //             eprintln!("检查房间状态失败: {}", e);
        //         }
        //     }
        // });

        while let Some(message) = stream.next().await.transpose()? {
            self.handle_message(message).await.expect("Error handling message");
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        match &message.command {
            Command::PRIVMSG(target, msg) => {
                println!("收到消息: {} <- {}", target, msg);
                if msg.starts_with("!") {
                    let prefix = self.get_nickname(&message.prefix);
                    handle_command(self, target, msg, prefix).await?;
                } else {
                    handle_event(self, target, msg).await?;
                }
            }
            Command::JOIN(channel, _, _) => {
                if let Some(nick) = self.get_nickname(&message.prefix) {
                    println!("{} joined {}", nick, channel);
                }
            }
            Command::PART(channel, _) => {
                if let Some(nick) = self.get_nickname(&message.prefix) {
                    if nick == "ATRI1024" {
                        // 延迟10秒后重新创建mp房间
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        self.create_room().await?;
                        println!("{} left {}", nick, channel);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn send_message(&self, target: &str, message: &str) -> Result<(), irc::error::Error> {
        self.client.send_privmsg(target, message)?;
        println!("发送消息: {} -> {}", target, message);
        Ok(())
    }

    pub async fn join_channel(&self, channel: &str) -> Result<(), irc::error::Error> {
        self.client.send_join(channel)
    }

    async fn check_room_status(room_id: u32) -> Result<(), Box<dyn Error>> {
        println!("Checking status of room: {}", room_id);
        // 这里你通常会使用 osu! API 检查房间状态
        // 现在我们只是打印一条消息
        Ok(())
    }

    pub async fn rotate_host(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(new_host) = self.room_host_list.pop() {
            self.room_host_list.insert(0, new_host.clone());
            self.set_host(&new_host).await?;
            println!("Rotated host to: {}", new_host);
        }
        Ok(())
    }

    fn get_nickname(&self, prefix: &Option<Prefix>) -> Option<String> {
        prefix.as_ref().and_then(|p| {
            if let Prefix::Nickname(nick, _, _) = p {
                Some(nick.to_string())
            } else {
                None
            }
        })
    }

    pub fn calculate_pp(&self, beatmap_id: u32, mods: u32, combo: u32, accuracy: f64) -> Result<(f64, f64, f64), Box<dyn Error>> {
        self.pp_calculator.calculate_pp(beatmap_id, mods, combo, accuracy, 0)
    }

    pub fn add_player(&mut self, name: String) {
        if !self.player_list.contains(&name) {
            self.player_list.push(name.clone());
        }
        if !self.room_host_list.contains(&name) {
            self.room_host_list.push(name);
        }
    }

    pub fn remove_player(&mut self, name: &str) {
        self.player_list.retain(|n| n != name);
    }

    pub fn remove_player_not_in_list(&mut self) {
        // 取player_list和room_host_list的交集，更新room_host_list
        self.room_host_list = self.room_host_list.iter()
            .filter(|player| self.player_list.contains(player))
            .cloned()
            .collect();
    }

    pub async fn create_room(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message("BanchoBot", "!mp make ATRI高性能mp房测试ver.").await?;
        println!("Sent room creation request to BanchoBot");
        
        // 等待一段时间,确保房间ID已经被设置
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // 保存房间ID到文件
        self.save_room_id_to_file().await?;
        
        Ok(())
    }

    pub async fn get_mp_settings(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), "!mp settings").await?;
        Ok(())
    }

    pub async fn set_room_password(&mut self, password: String) -> Result<(), Box<dyn Error>> {
        let room_id = *self.room_id.lock().await;
        self.send_message(&format!("#mp_{}", room_id), &format!("!mp password {}", password)).await?;
        Ok(())
    }

    pub async fn calculate_total_time_left(&self) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.beatmap_start_time.unwrap_or(now));
        if elapsed == Duration::from_secs(0) {
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await), "游戏尚未开始").await?;
            return Ok(());
        }
        else {
            let total_time_left = self.beatmap_length - elapsed.as_secs();
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("剩余游玩时间: {}s", total_time_left)).await?;
        }
        Ok(())
    }

    pub async fn set_host(&mut self, player_name: &str) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("!mp host {}", player_name)).await?;
        self.room_host = player_name.to_string();
        Ok(())
    }

    pub async fn set_free_mod(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), "!mp mods FreeMod").await?;
        Ok(())
    }

    pub async fn start_game(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), "!mp start").await?;
        Ok(())
    }

    pub async fn abort_game(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), "!mp abort").await?;
        Ok(())
    }

    pub async fn close_room(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), "!mp close").await?;
        Ok(())
    }

    pub async fn send_queue(&mut self) -> Result<(), Box<dyn Error>> {
        let queue = self.player_list.iter()
            .map(|name| format!("\u{200B}{}", name))
            .collect::<Vec<String>>()
            .join("->");
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &queue).await?;
        Ok(())
    }

    pub async fn get_user_mut(&mut self, irc_name: &str) -> Option<&mut User> {
        if !self.player_info.contains_key(irc_name) {
            let mut user = User::new(irc_name.to_string(), 0, "".to_string());
            user.update(&mut self.osu_api).await.unwrap();
            self.player_info.insert(irc_name.to_string(), user);
        }
        self.player_info.get_mut(irc_name)
    }

    pub async fn send_beatmap_info(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &self.beatmap_info).await?;
        Ok(())
    }

    pub async fn vote_skip(&mut self, irc_name: &str) -> Result<(), Box<dyn Error>> {
        // 判断irc_name是否在player_list中
        if self.player_list.contains(&irc_name.to_string()) {
            // 如果不在approved_skip_list中，则添加到approved_skip_list中
            if !self.approved_skip_list.contains(&irc_name.to_string()) {
                self.approved_skip_list.push(irc_name.to_string());
            }
        // 判断列表是否满足人数的一半 或者是房主本人
        if self.approved_skip_list.len() >= (self.player_list.len() / 2) || irc_name == self.room_host.replace(" ", "_") {
            self.rotate_host().await?;
            self.approved_skip_list.clear();
        }
        else {
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("{} / {} in the skip process", self.approved_skip_list.len(), (self.player_list.len() as f64 / 2.0).ceil() as usize)).await?;
        }
    }
        Ok(())
    }
    pub async fn vote_close(&mut self, irc_name: &str) -> Result<(), Box<dyn Error>> {
        // 判断irc_name是否在player_list中
        if self.player_list.contains(&irc_name.to_string()) {
            // 如果不在approved_close_list中，则添加到approved_close_list中
            if !self.approved_close_list.contains(&irc_name.to_string()) {
                self.approved_close_list.push(irc_name.to_string());
            }
        }
        // 判断列表是否满足人数的一半
        if self.approved_close_list.len() >= (self.player_list.len() / 2) {
            self.close_room().await?;
            self.approved_close_list.clear();
        }
        else {
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("{} / {} in the close process", self.approved_close_list.len(), (self.player_list.len() as f64 / 2.0).ceil() as usize)).await?;
        }
        Ok(())
    }
    pub async fn vote_start(&mut self, irc_name: &str) -> Result<(), Box<dyn Error>> {
        // 判断irc_name是否在player_list中
        if self.player_list.contains(&irc_name.to_string()) {
            // 如果不在approved_start_list中，则添加到approved_start_list中
            if !self.approved_start_list.contains(&irc_name.to_string()) {
                self.approved_start_list.push(irc_name.to_string());
            }
        }
        // 判断列表是否满足人数的一半
        if self.approved_start_list.len() >= (self.player_list.len() / 2) {
            self.start_game().await?;
            self.approved_start_list.clear();
        }
        else {
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("{} / {} in the start process", self.approved_start_list.len(), (self.player_list.len() as f64 / 2.0).ceil() as usize)).await?;
        }
        Ok(())
    }

    pub async fn save_room_id_to_file(&self) -> Result<(), Box<dyn Error>> {
        let room_id = *self.room_id.lock().await;
        let mut file = File::create("last_room_id.txt")?;
        write!(file, "{}", room_id)?;
        println!("Room ID {} saved to last_room_id.txt", room_id);
        Ok(())
    }
}
