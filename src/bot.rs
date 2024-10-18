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
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

pub struct MyBot {
    client: Client,
    pub player_list: Vec<String>,
    pub room_host_list: Vec<String>,
    pub approved_abort_list: Vec<String>,
    pub approved_start_list: Vec<String>,
    pub approved_host_rotate_list: Vec<String>,
    pub approved_close_list: Vec<String>,
    pub room_host: String,
    pub room_id: Arc<TokioMutex<u32>>,
    pub game_start_time: Option<Instant>,
    pub beatmap_id: u32,
    pub beatmap_length: u64,
    pub beatmap_path: String,
    pp_calculator: PPCalculator,
    osu_api: OsuApi,
}

impl MyBot {
    pub async fn new(config: Config, client_id: String, client_secret: String) -> Result<Self, irc::error::Error> {
        let client = Client::from_config(config).await?;
        Ok(MyBot {
            client,
            player_list: Vec::new(),
            room_host_list: Vec::new(),
            approved_abort_list: Vec::new(),
            approved_start_list: Vec::new(),
            approved_host_rotate_list: Vec::new(),
            approved_close_list: Vec::new(),
            room_host: String::new(),
            room_id: Arc::new(TokioMutex::new(0)),
            game_start_time: None,
            beatmap_id: 0,
            beatmap_length: 0,
            beatmap_path: String::new(),
            pp_calculator: PPCalculator::new(String::new()),
            osu_api: OsuApi::new(client_id, client_secret),
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.client.identify()?;

        self.create_room().await?;

        let mut stream = self.client.stream()?;

        // Start the periodic task
        let room_id = Arc::clone(&self.room_id);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let current_room_id = *room_id.lock().await;
                if let Err(e) = Self::check_room_status(current_room_id).await {
                    eprintln!("Error checking room status: {}", e);
                }
            }
        });

        while let Some(message) = stream.next().await.transpose()? {
            self.handle_message(message).await?;
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        match &message.command {
            Command::PRIVMSG(target, msg) => {
                println!("Received message in {}: {}", target, msg);
                if msg.starts_with("!") {
                    let prefix = self.get_nickname(&message.prefix);
                    handle_command(self, target, msg, prefix).await?;
                } else if target == "ATRI1024" && msg.contains("Created the tournament match") {
                    self.parse_room_id(msg).await?;
                } else if msg.contains("Beatmap changed to") {
                    self.handle_beatmap_change(msg).await?;
                } else if msg.contains("The match has started") {
                    self.handle_match_start().await?;
                } else if msg.contains("The match has finished") {
                    self.handle_match_finish().await?;
                }
            }
            Command::JOIN(channel, _, _) => {
                if let Some(nick) = self.get_nickname(&message.prefix) {
                    println!("{} joined {}", nick, channel);
                    self.add_player(nick);
                }
            }
            Command::PART(channel, _) => {
                if let Some(nick) = self.get_nickname(&message.prefix) {
                    println!("{} left {}", nick, channel);
                    self.remove_player(&nick);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn parse_room_id(&mut self, msg: &str) -> Result<(), Box<dyn Error>> {
        let re = Regex::new(r"https://osu\.ppy\.sh/mp/(\d+)")?;
        if let Some(captures) = re.captures(msg) {
            if let Some(id) = captures.get(1) {
                let new_room_id = id.as_str().parse::<u32>()?;
                {
                    let mut room_id = self.room_id.lock().await;
                    *room_id = new_room_id;
                }
                println!("Room ID set to: {}", new_room_id);
                self.join_channel(&format!("#mp_{}", new_room_id)).await?;
                self.set_room_password("123").await?;
            }
        }
        Ok(())
    }

    async fn handle_beatmap_change(&mut self, msg: &str) -> Result<(), Box<dyn Error>> {
        let re = Regex::new(r"Beatmap changed to: (.*) \((https://osu\.ppy\.sh/b/(\d+))\)")?;
        if let Some(captures) = re.captures(msg) {
            if let Some(id) = captures.get(3) {
                self.beatmap_id = id.as_str().parse::<u32>()?;
                println!("Beatmap ID changed to: {}", self.beatmap_id);
                
                // 获取谱面信息
                let beatmap = self.osu_api.get_beatmap(self.beatmap_id).await?;
                
                // 更新 beatmap_path
                self.beatmap_path = format!("./maps/{}.osu", self.beatmap_id);
                self.pp_calculator = PPCalculator::new(self.beatmap_path.clone());

                let mods = 0;
                let (stars, pp, max_pp, pp_95_fc, pp_96_fc, pp_97_fc, pp_98_fc, pp_99_fc) = self.pp_calculator.calculate_beatmap_details(mods)?;

                let pp_info = format!("Stars: {:.2} | 95%: {:.2}pp | 98%: {:.2}pp | 100%: {:.2}pp | Max: {:.2}pp", 
                                      stars, pp_95_fc, pp_98_fc, max_pp, max_pp);
                
                let beatmap_info = format!("Beatmap: {} [{}] | Length: {}s | Status: {}", 
                                           beatmap.beatmapset_id, beatmap.version, beatmap.total_length, beatmap.status);
                
                self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &beatmap_info).await?;
                self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &pp_info).await?;
            }
        }
        Ok(())
    }

    async fn handle_match_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.game_start_time = Some(Instant::now());
        println!("Match started");
        Ok(())
    }

    async fn handle_match_finish(&mut self) -> Result<(), Box<dyn Error>> {
        self.game_start_time = None;
        println!("Match finished");
        self.rotate_host().await?;
        Ok(())
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
            let room_id = *self.room_id.lock().await;
            self.send_message(&format!("#mp_{}", room_id), &format!("!mp host {}", new_host)).await?;
            self.room_host = new_host;
            println!("Rotated host to: {}", self.room_host);
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

    pub async fn send_message(&self, target: &str, message: &str) -> Result<(), irc::error::Error> {
        self.client.send_privmsg(target, message)
    }

    pub async fn join_channel(&self, channel: &str) -> Result<(), irc::error::Error> {
        self.client.send_join(channel)
    }

    async fn create_room(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message("BanchoBot", "!mp make ATRI1024's room").await?;
        println!("Sent room creation request to BanchoBot");
        Ok(())
    }

    async fn set_room_password(&mut self, password: &str) -> Result<(), Box<dyn Error>> {
        let room_id = *self.room_id.lock().await;
        self.send_message(&format!("#mp_{}", room_id), &format!("!mp password {}", password)).await?;
        Ok(())
    }

    pub fn calculate_pp(&self, mods: u32, combo: u32, accuracy: f64) -> Result<(f64, f64, f64), Box<dyn Error>> {
        self.pp_calculator.calculate_pp(mods, combo, accuracy, 0)
    }
}
