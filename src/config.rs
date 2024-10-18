use irc::client::prelude::Config;
use std::env;

pub fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let password = env::var("IRC_PASSWORD").expect("IRC_PASSWORD must be set");

    Ok(Config {
        nickname: Some("ATRI1024".to_owned()),
        server: Some("irc.ppy.sh".to_owned()),
        port: Some(6667),
        channels: vec!["#welcome".to_owned()],
        password: Some(password),
        use_tls: Some(false),
        ..Config::default()
    })
}