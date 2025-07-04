mod crawler;
mod fetcher;
mod parser;
mod storage;

use std::time::Instant;

use clap::Parser as ClapParser;
use tracing::Level;
use tracing::info;
use tracing_subscriber::fmt;

use crate::crawler::{Crawler, CrawlerConfig};

#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    url: String,

    #[arg(short, long, default_value_t = 2)]
    depth: usize,

    #[arg(short, long, default_value_t = 50)]
    max_pages: usize,

    #[arg(short, long, default_value_t = 10)]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fmt::fmt().with_max_level(Level::INFO).init();

    let args = Args::parse();

    let config = CrawlerConfig {
        max_depth: args.depth,
        max_pages_per_domain: args.max_pages,
        concurrent_requests: args.concurrency,
        ..Default::default()
    };

    let crawler = Crawler::new(config);
    let start_time = Instant::now();

    info!("Starting crawl at {} with depth {}", args.url, args.depth);

    let pages = crawler.crawl(&args.url).await?;
    let elapsed = start_time.elapsed();

    info!(
        "Crawl complete! Fetched {} pages in {:.2?}",
        pages.len(),
        elapsed
    );

    if !pages.is_empty() {
        let mut domain_counts = std::collections::HashMap::new();

        for page in &pages {
            if let Some(host) = page.url.host_str() {
                let count = domain_counts.entry(host.to_string()).or_insert(0);
                *count += 1;
            }
        }

        info!("Pages per domain:");
        for (domain, count) in domain_counts {
            info!("  {}: {}", domain, count);
        }
    }

    Ok(())
}
