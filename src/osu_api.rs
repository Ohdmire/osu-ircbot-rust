use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct OsuApi {
    client: Client,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u32,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub irc_name: String,
    pub id: u32,
    pub username: String,
}

#[derive(Deserialize, Debug)]
pub struct UserData {
    pub id: u32,
    pub username: String,
}

#[derive(Deserialize, Debug)]
pub struct Beatmap {
    pub id: u32,
    pub beatmapset_id: u32,
    pub status: String,
    pub total_length: u32,
    pub version: String,
    pub difficulty_rating: f32,
    pub accuracy: f32,
    pub ar: f32,
    pub bpm: f32,
    pub cs: f32,
    pub drain: f32,
    pub mode_int: u32,
    pub max_combo: u32,
    pub beatmapset: Beatmapset,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct Beatmapset {
    pub artist: String,
    pub title: String,
    pub title_unicode: String,
    pub artist_unicode: String,
    pub submitted_date: String,
    pub ranked_date: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UserScore {
    pub score: Score,
}

#[derive(Deserialize, Debug)]
pub struct Score {
    pub accuracy: f64,
    pub best_id: Option<u64>,
    pub created_at: String,
    pub id: u64,
    pub max_combo: u32,
    pub mode: String,
    pub mode_int: u8,
    pub mods: Vec<String>,
    pub passed: bool,
    pub perfect: bool,
    pub pp: f32,
    pub rank: String,
    pub replay: bool,
    pub score: u64,
    pub statistics: ScoreStatistics,
}

#[derive(Deserialize, Debug)]
pub struct ScoreStatistics {
    pub count_100: u32,
    pub count_300: u32,
    pub count_50: u32,
    pub count_geki: Option<u32>,
    pub count_katu: Option<u32>,
    pub count_miss: u32,
}

impl User {
    pub fn new(irc_name: String, id: u32, username: String) -> Self {
        Self { irc_name, id, username }
    }

    pub async fn update(&mut self, osu_api: &mut OsuApi) -> Result<(), Box<dyn Error>> {
        let userdata = osu_api.get_user_info(self.irc_name.clone()).await.unwrap();
        self.id = userdata.id;
        self.username = userdata.username;
        Ok(())
    }
}

impl Beatmap {
    pub fn get_formatted_info(&self) -> String {
        let date = self.beatmapset.ranked_date
            .as_deref()
            .or(Some(&self.beatmapset.submitted_date))
            .and_then(|date_str| {
                DateTime::parse_from_rfc3339(date_str)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc).format("%Y-%m-%d").to_string())
            })
            .unwrap_or_else(|| "Unknown Date".to_string());

        let length_seconds = self.total_length;
        let osudirect_url = self.url.clone();
        let sayo_url = format!("https://osu.sayobot.cn/home?search={}", self.beatmapset_id);
        let inso_url = format!("http://inso.link/yukiho/?b={}", self.beatmapset_id);

        format!(
            "{} {}| {}*| [{} {} - {}]| bpm:{} length:{}s| ar:{} cs:{} od:{} hp:{}| [{} Sayobot] OR [{} inso]",
            date, self.status, self.difficulty_rating, osudirect_url,
            self.beatmapset.title_unicode, self.beatmapset.artist_unicode, self.bpm, length_seconds,
            self.ar, self.cs, self.accuracy, self.drain, sayo_url, inso_url
        )
    }
}

impl OsuApi {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            access_token: None,
        }
    }

    async fn get_token(&mut self) -> Result<(), Box<dyn Error>> {
        let params = [
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("grant_type", &"client_credentials".to_string()),
            ("scope", &"public".to_string()),
        ];

        let res: TokenResponse = self.client
            .post("https://osu.ppy.sh/oauth/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        self.access_token = Some(res.access_token);
        Ok(())
    }

    pub async fn get_user_info(&mut self, irc_name: String) -> Result<UserData, Box<dyn Error>> {
        self.get_token().await?;

        let url = format!("https://osu.ppy.sh/api/v2/users/{}", irc_name);
        let res = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token.as_ref().unwrap()))
            .send()
            .await?;
        if res.status().is_success() {
            let userdata: UserData = res.json().await?;
            Ok(userdata)
        } else {
            Err(format!("Failed to get user info: {:?}", res.status()).into())
        }
    }

    pub async fn get_beatmap_info(&mut self, beatmap_id: u32) -> Result<Beatmap, Box<dyn Error>> {
        self.get_token().await?;

        let url = format!("https://osu.ppy.sh/api/v2/beatmaps/{}", beatmap_id);
        let res = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token.as_ref().unwrap()))
            .send()
            .await?;

        if res.status().is_success() {
            let beatmap: Beatmap = res.json().await?;
            Ok(beatmap)
        } else {
            Err(format!("Failed to get beatmap: {:?}", res.status()).into())
        }
    }

    pub async fn download_beatmap(&mut self, beatmap_id: u32) -> Result<(), Box<dyn Error>> {

        // 检查文件是否已存在
        let file_path = format!("./maps/{}.osu", beatmap_id);
        let path = Path::new(&file_path);
        if path.exists() {
            println!("Beatmap already exists: {}", file_path);
            return Ok(());
        }

        let url = format!("https://osu.direct/api/osu/{}", beatmap_id);
        let res = self.client
            .get(&url)
            .send()
            .await?;

        if res.status().is_success() {
            let bytes = res.bytes().await?;
            let file_path = format!("./maps/{}.osu", beatmap_id);
            let path = Path::new(&file_path);

            // 确保 maps 目录存在
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut file = File::create(path)?;
            file.write_all(&bytes)?;
            println!("Beatmap downloaded and saved to: {}", file_path);
            Ok(())
        } else {
            Err(format!("Failed to download beatmap: {:?}", res.status()).into())
        }
    }

    pub async fn get_user_score(&mut self, user_id: u32, beatmap_id: u32) -> Result<UserScore, Box<dyn Error>> {
        self.get_token().await?;
        
        let url = format!("https://osu.ppy.sh/api/v2/beatmaps/{}/scores/users/{}", beatmap_id, user_id);
        let res = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token.as_ref().unwrap()))
            .send()
            .await?;

        if res.status().is_success() {
            let user_score: UserScore = res.json().await?;
            Ok(user_score)
        } else {
            Err(format!("Failed to get user score: {:?}", res.status()).into())
        }
    }
    // Add more API methods as needed
}
