use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::lock::Mutex;
use reqwest::Client;
use url::Url;

use crate::crawler::{CrawlerConfig, CrawlerError, CrawlerResult};

pub struct Fetcher {
    client: Client,
    config: CrawlerConfig,
    last_access: Arc<Mutex<HashMap<String, Instant>>>,
}

impl Fetcher {
    pub fn new(config: CrawlerConfig) -> Self {
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            last_access: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn fetch(&self, url: &Url) -> CrawlerResult<String> {
        let host = url
            .host_str()
            .ok_or_else(|| CrawlerError::UrlParseError(url::ParseError::EmptyHost))?;

        {
            let mut access_times = self.last_access.lock().await;
            let now = Instant::now();

            if let Some(last_time) = access_times.get(host) {
                let elapsed = now.duration_since(*last_time);
                let min_delay = Duration::from_millis(self.config.delay_ms);

                if elapsed < min_delay {
                    let sleep_time = min_delay - elapsed;
                    tokio::time::sleep(sleep_time).await;
                }
            }

            access_times.insert(host.to_string(), Instant::now());
        }

        let response = self.client.get(url.as_str()).send().await?;
        let response = response.error_for_status()?;

        Ok(response.text().await?)
    }
}
