use crate::{pp_calculator, BotSettings};
use crate::osu_api::{self, User};

use crate::charts;

use irc::client::prelude::*;

use std::error::Error;
use futures::stream::StreamExt;
use std::time::{Instant, Duration};
use crate::commands::handle_command;

use self::pp_calculator::PPCalculator;
use self::osu_api::OsuApi;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use crate::events::handle_event;

use std::collections::HashMap;

use std::fs::File;
use std::io::{Write, Read};

use serde::{Serialize, Deserialize};
use crate::charts::ChartDatabase;

#[derive(Serialize, Deserialize)]
struct BotState {
    beatmap_name: String,
    beatmap_artist: String,
    beatmap_star: f32,
    player_list: Vec<String>,
}

pub struct MyBot {
    client: Client,
    pub chart_db :ChartDatabase,
    pub bot_name: String,
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
    pub is_channel_exist: bool,
    pub is_game_started: bool,
}

impl MyBot {
    pub async fn new(config: Config, client_id: String, client_secret: String,bot_settings: BotSettings) -> Result<Self, Box<dyn Error>> {
        let nickname = config.nickname.clone();
        let client = Client::from_config(config).await?;
        
        // 尝试读取上次保存的房间ID
        let last_room_id = Self::read_last_room_id().unwrap_or(0);
        
        let bot = MyBot {
            client,
            chart_db: ChartDatabase::open("charts.sqlite").unwrap(),
            bot_name: nickname.unwrap(),
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
            room_name: bot_settings.room_name,
            room_password: bot_settings.room_password,
            beatmap_id: 0,
            is_channel_exist: false,
            is_game_started:false,
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

        let mut stream = self.client.stream()?;

        let room_id = *self.room_id.lock().await;
        if room_id == 0 {
            // 如果没有上次的房间ID,创建新房间
            self.create_room().await?;
        } else {
            println!("Using existing room: #mp_{}", room_id);
            // 尝试加入上次的房间
            self.join_last_room().await?;
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await), "!mp settings").await?;
        }

        while let Some(message) = stream.next().await.transpose()? {
            match self.handle_message(message).await {
                Ok(_) => {},
                Err(e) => {
                    println!("Error handling message: {:?}", e);
                }
            }
        }


        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        match &message.command {
            Command::PRIVMSG(target, msg) => {
                let sender = self.get_nickname(&message.prefix).unwrap_or("unknown".to_string());
                println!("收到消息: {} <- {} from {}", target, msg,sender);
                if msg.contains("Match settings") {
                    self.is_channel_exist = true;
                }
                if msg.starts_with("help"){
                    self.send_menu().await?;
                }
                if msg.starts_with("!") || msg.starts_with("！") {
                    let prefix = self.get_nickname(&message.prefix);
                    handle_command(self, &sender,target, msg, prefix).await?;
                } else {
                    handle_event(self, &sender, msg).await?;
                }
            }
            Command::JOIN(channel, _, _) => {
                if let Some(nick) = self.get_nickname(&message.prefix) {
                    println!("{} joined {}", nick, channel);
                }
            }
            Command::PART(channel, _) => {
                if let Some(nick) = self.get_nickname(&message.prefix) {
                    println!("{} left {}", nick, channel);
                    if nick == self.bot_name {
                        println!("Bot was kicked from the channel");
                        // 清空队列
                        self.player_list.clear();
                        self.save_latest_info_to_file().expect("无法写入bot state");
                        // 退出终止进程
                        !panic!("End.")
                    }
                }
            }
            
            Command::Response(Response::ERR_NOSUCHCHANNEL,args) => {
                self.is_channel_exist = false;          
                println!("{:?},{:?}",Response::ERR_NOSUCHCHANNEL,args);       
                println!("Not found channel, Recreate");
                self.create_room().await?;
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

    pub async fn rotate_host(&mut self) -> Result<(), Box<dyn Error>> {
        //轮换房主前，删除不在player_list中的玩家
        self.remove_player_not_in_list();
        if !self.room_host_list.is_empty() {
            let old_host = self.room_host_list.remove(0);
            self.room_host_list.push(old_host);
            let new_host = self.room_host_list[0].clone();
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
        self.send_message("BanchoBot", &format!("!mp make {}", self.room_name)).await?;
        println!("Sent room creation request to BanchoBot");
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

    pub async fn calculate_total_time_left(&self) -> Result<(String), Box<dyn Error>> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.beatmap_start_time.unwrap_or(now));
        if elapsed == Duration::from_secs(0) {
            let msg_not_started = "游戏尚未开始".to_string();
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &msg_not_started).await?;
            Ok(msg_not_started)
        }
        else {
            let total_time_left = self.beatmap_length - elapsed.as_secs();
            let msg_started = format!("剩余游玩时间: {}s", total_time_left);
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await),&msg_started).await?;
            Ok(msg_started)
        }
    }

    pub async fn send_welcome(&mut self, player_name: String) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await),&format!("欢迎{}酱~＼(≧▽≦)／ 输入help获取指令详情", player_name)).await?;

        if self.is_game_started{
            let remain_time_text = self.calculate_total_time_left().await?;
            self.send_message(&format!("#mp_{}", *self.room_id.lock().await),&remain_time_text).await?;
        }

        Ok(())
    }

    pub async fn send_menu(&mut self) -> Result<(), Box<dyn Error>> {
        let help_text = "!queue(!q) 查看队列 | !abort 投票丢弃游戏 | !start 投票开始游戏 | !skip 投票跳过房主 | !pr(!p) 查询最近pass成绩 | !re(!r) 查询最近成绩 | !s 查询当前谱面最好成绩| !info(!i) 返回当前谱面信息| !pick 挑选一张赛图| !ttl 查询剩余时间 | help(!h) 查看帮助 | !about 关于机器人";
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await),help_text).await?;
        Ok(())
    }

    pub async fn send_about(&mut self) -> Result<(), Box<dyn Error>> {
        let about_text = "https://github.com/Ohdmire/osu-ircbot-rust ATRI高性能bot with Rust";
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await),about_text).await?;
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

    pub async fn set_map(&mut self,map_id: i32) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("!mp map {}", map_id.to_string())).await?;
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
    
    pub async fn cleanup_after_match(&mut self) -> Result<(), Box<dyn Error>> {
        self.approved_abort_list.clear();
        self.approved_skip_list.clear();
        self.approved_start_list.clear();
        self.approved_close_list.clear();
        Ok(())
    }

    pub async fn send_queue(&mut self) -> Result<(), Box<dyn Error>> {
        let queue = self.room_host_list.iter()
            .map(|name| {
                name.chars()
                .map(|c| format!("{c}\u{200B}"))
                .collect::<String>()
                .trim_end_matches('\u{200B}')
                .to_owned()
            })
                .collect::<Vec<_>>()
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
        self.save_latest_info_to_file().expect("无法写入bot state");
        Ok(())
    }
    
    pub async fn vote_abort(&mut self, irc_name: &str) -> Result<(), Box<dyn Error>> {
        // 判断irc_name是否在player_list中
        if self.player_list.contains(&irc_name.to_string()) {
            // 如果不在approved_abort_list中，则添加到approved_abort_list中
            if !self.approved_abort_list.contains(&irc_name.to_string()) {
                self.approved_abort_list.push(irc_name.to_string());
            }

            // 判断列表是否满足人数的一半 或者是房主本人
            if self.approved_abort_list.len() >= (self.player_list.len() / 2) || irc_name == self.room_host.replace(" ", "_") {
                self.abort_game().await?;
                self.approved_abort_list.clear();
            }
            else {
                self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("{} / {} in the abort process", self.approved_abort_list.len(), (self.player_list.len() as f64 / 2.0).ceil() as usize)).await?;
            }
        }
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

    pub fn save_latest_info_to_file(&self) -> Result<(), Box<dyn Error>> {
        let state = BotState{
            beatmap_name: self.beatmap_title_unicode.clone(),
            beatmap_artist: self.beatmap_artist_unicode.clone(),
            beatmap_star: self.beatmap_difficulty_rating,
            player_list: self.player_list.clone()
        };
        let mut file = File::create("bot_state.json")?;
        serde_json::to_writer_pretty(&file, &state)?;
        Ok(())
    }
}
