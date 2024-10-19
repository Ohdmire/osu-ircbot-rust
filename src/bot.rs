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
    pub room_password: String,
    pub game_start_time: Option<Instant>,
    pub beatmap_id: u32,
    pub beatmap_length: u64,
    pub beatmap_path: String,
    pub pp_calculator: PPCalculator,
    pub osu_api: OsuApi,
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
            room_password: env::var("ROOM_PASSWORD").unwrap_or_else(|_| "".to_string()),
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
                } else {
                    handle_event(self, target, msg).await?;
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

    pub async fn create_room(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_message("BanchoBot", "!mp make ATRI1024's room").await?;
        println!("Sent room creation request to BanchoBot");
        Ok(())
    }

    pub async fn set_room_password(&mut self, password: String) -> Result<(), Box<dyn Error>> {
        let room_id = *self.room_id.lock().await;
        self.send_message(&format!("#mp_{}", room_id), &format!("!mp password {}", password)).await?;
        Ok(())
    }

    pub fn calculate_pp(&self, mods: u32, combo: u32, accuracy: f64) -> Result<(f64, f64, f64), Box<dyn Error>> {
        self.pp_calculator.calculate_pp(mods, combo, accuracy, 0)
    }

    pub async fn set_host(&mut self, player_name: &str) -> Result<(), Box<dyn Error>> {
        self.send_message(&format!("#mp_{}", *self.room_id.lock().await), &format!("!mp host {}", player_name)).await?;
        Ok(())
    }
}
