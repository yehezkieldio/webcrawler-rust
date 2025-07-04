use thiserror::Error;
use url::Url;

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
