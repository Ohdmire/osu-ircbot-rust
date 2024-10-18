use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

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

#[derive(Deserialize)]
pub struct Beatmap {
    pub id: u32,
    pub beatmapset_id: u32,
    pub status: String,
    pub total_length: u32,
    pub version: String,
    // Add more fields as needed
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
