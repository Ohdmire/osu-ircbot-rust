use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use chrono::{DateTime, Utc, TimeZone};

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
        let osudirect_url = format!("https://osu.ppy.sh/beatmapsets/{}", self.beatmapset_id);
        let sayo_url = format!("https://osu.sayobot.cn/osu.php?s={}", self.beatmapset_id);
        let inso_url = format!("https://inso.link/d/{}", self.beatmapset_id);

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

    pub async fn get_beatmap_info(&mut self, beatmap_id: u32) -> Result<Beatmap, Box<dyn Error>> {
        if self.access_token.is_none() {
            self.get_token().await?;
        }

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

    // Add more API methods as needed
}
