use getset::Getters;

#[derive(Clone, Debug, Builder, Getters)]
#[getset(get = "pub")]
#[builder(default)]
pub struct AggregatorConfig {
    http: HttpConfig,
    telegram: TelegramConfig,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            http: HttpConfig::default(),
            telegram: TelegramConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Builder, Getters)]
#[getset(get = "pub")]
#[builder(default)]
pub struct HttpConfig {
    enabled: bool,
    sleep_secs: u64,
    scrape_source_secs_interval: i32,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sleep_secs: 60,
            scrape_source_secs_interval: 60,
        }
    }
}

#[derive(Clone, Debug, Builder, Getters)]
#[getset(get = "pub")]
pub struct TelegramConfig {
    enabled: bool,
    database_directory: String,
    log_verbosity_level: i32,
    api_id: i64,
    api_hash: String,
    phone: String,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            database_directory: "tdlib".to_string(),
            log_verbosity_level: 0,
            api_id: 0,
            api_hash: "".to_string(),
            phone: "".to_string(),
        }
    }
}
