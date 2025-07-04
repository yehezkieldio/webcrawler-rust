use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{Semaphore, mpsc};
use tracing::{error, info, warn};
use url::Url;

use crate::{fetcher::Fetcher, parser::Parser, storage::Storage};

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Network error occurred: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Failed to parse the HTML content: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Input/Output error occurred: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Max depth reached")]
    MaxDepthReached,

    #[error("Max pages limit reached: {0}")]
    RateLimitExceeded(String),
}

pub type CrawlerResult<T> = Result<T, CrawlerError>;

#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    pub max_depth: usize,
    pub max_pages_per_domain: usize,
    pub respect_robots_txt: bool,
    pub concurrent_requests: usize,
    pub user_agent: String,
    pub delay_ms: u64,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_pages_per_domain: 100,
            respect_robots_txt: true,
            concurrent_requests: 5,
            user_agent: "WebCrawlerRust/0.1.0".to_string(),
            delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub url: Url,
    pub depth: usize,
    pub content: String,
    pub links: Vec<String>,
}

pub struct Crawler {
    config: CrawlerConfig,
    fetcher: Arc<Fetcher>,
    parser: Arc<Parser>,
    storage: Arc<Storage>,
}

impl Crawler {
    pub fn new(config: CrawlerConfig) -> Self {
        let fetcher = Arc::new(Fetcher::new(config.clone()));
        let parser = Arc::new(Parser::new());
        let storage = Arc::new(Storage::new());

        Self {
            config,
            fetcher,
            parser,
            storage,
        }
    }

    pub async fn crawl(&self, start_url: &str) -> CrawlerResult<Vec<Page>> {
        let start_url = Url::parse(start_url)?;
        let (tx, mut rx) = mpsc::channel(1000);

        tx.send((start_url, 0))
            .await
            .expect("Channel should be open");

        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_requests));

        while let Some((url, depth)) = rx.recv().await {
            if depth > self.config.max_depth {
                continue;
            }

            if self.storage.is_visited(&url) {
                continue;
            }

            self.storage.mark_visited(&url);

            if let Some(host) = url.host_str() {
                if self.storage.domain_page_count(host).await >= self.config.max_pages_per_domain {
                    warn!("Skipping {}: reached max pages for domain", url);
                    continue;
                }
            }

            let url_clone = url.clone();
            let tx_clone = tx.clone();
            let fetcher = self.fetcher.clone();
            let parser = self.parser.clone();
            let storage = self.storage.clone();
            let semaphore_clone = semaphore.clone();

            let max_depth = self.config.max_depth;

            tokio::spawn(async move {
                let permit = semaphore_clone
                    .acquire()
                    .await
                    .expect("Semaphore should not be closed");

                let result = async {
                    info!("Fetching {}", url_clone);

                    let content = fetcher.fetch(&url_clone).await?;
                    let page = parser.parse(&url_clone, content, depth)?;

                    storage.store_page(page.clone()).await;

                    if depth < max_depth {
                        for link in &page.links {
                            if let Ok(parsed_url) = Url::parse(link) {
                                tx_clone
                                    .send((parsed_url, depth + 1))
                                    .await
                                    .expect("Channel should be open");
                            } else {
                                warn!("Invalid URL found: {}", link);
                            }
                        }
                    }

                    Ok::<_, CrawlerError>(())
                }
                .await;

                if let Err(err) = result {
                    error!("Error processing {}: {:?}", url_clone, err);
                }

                drop(permit);
            });
        }

        Ok(self.storage.get_pages().await)
    }
}
