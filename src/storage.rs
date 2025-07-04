use std::{collections::HashMap, sync::Arc};

use dashmap::DashSet;
use futures::lock::Mutex;
use url::Url;

use crate::crawler::Page;

pub struct Storage {
    visited_urls: DashSet<String>,
    pages: Arc<Mutex<Vec<Page>>>,
    pages_per_domain: Arc<Mutex<HashMap<String, usize>>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            visited_urls: DashSet::new(),
            pages: Arc::new(Mutex::new(Vec::new())),
            pages_per_domain: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn is_visited(&self, url: &Url) -> bool {
        self.visited_urls.contains(url.as_str())
    }

    pub fn mark_visited(&self, url: &Url) -> bool {
        self.visited_urls.insert(url.as_str().to_string())
    }

    pub async fn store_page(&self, page: Page) {
        if let Some(host) = page.url.host_str() {
            let mut counts = self.pages_per_domain.lock().await;
            let count = counts.entry(host.to_string()).or_insert(0);
            *count += 1;
        }

        let mut pages = self.pages.lock().await;
        pages.push(page);
    }

    pub async fn domain_page_count(&self, domain: &str) -> usize {
        let counts = self.pages_per_domain.lock().await;
        *counts.get(domain).unwrap_or(&0)
    }

    pub async fn get_pages(&self) -> Vec<Page> {
        let pages = self.pages.lock().await;
        pages.clone()
    }
}
