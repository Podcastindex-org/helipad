use reqwest;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::error::Error;

use std::num::NonZeroUsize;
use lru::LruCache;

#[derive(Clone, Debug)]
pub struct PodcastEpisodeGuid {
    pub podcast_guid: String,
    pub episode_guid: String,
    pub podcast: Option<String>,
    pub episode: Option<String>,
}

pub struct GuidCache {
    pub cache: LruCache<String, PodcastEpisodeGuid>,
}

impl GuidCache {
    pub fn new(size: usize) -> GuidCache {
        GuidCache {
            cache: LruCache::new(NonZeroUsize::new(size).unwrap()),
        }
    }

    // Fetches remote podcast/episode names by guids using the Podcastindex API and caches results into an LRU cache
    pub async fn get(&mut self, podcast_guid: String, episode_guid: String) -> Result<PodcastEpisodeGuid, Box<dyn Error>> {
        let key = format!("{}_{}", podcast_guid, episode_guid);

        if let Some(cached_guid) = self.cache.get(&key) {
            println!("Remote podcast/episode from cache: {:#?}", cached_guid);
            return Ok(cached_guid.clone()); // already exists in cache
        }

        let guid = fetch_api_podcast_episode_by_guid(&podcast_guid, &episode_guid).await?;

        println!("Remote podcast/episode from API: {:#?}", guid);
        self.cache.put(key, guid.clone()); // cache to avoid spamming api

        Ok(guid)
    }
}

// Fetches remote podcast/episode names by guids using the Podcastindex API
pub async fn fetch_api_podcast_episode_by_guid(podcast_guid: &str, episode_guid: &str) -> Result<PodcastEpisodeGuid, Box<dyn Error>> {
    let query = vec![
        ("podcastguid", podcast_guid),
        ("episodeguid", episode_guid)
    ];

    let mut guid = PodcastEpisodeGuid {
        podcast_guid: podcast_guid.to_string(),
        episode_guid: episode_guid.to_string(),
        podcast: None,
        episode: None,
    };

    let app_version = env!("CARGO_PKG_VERSION");

    // call API, get text response, and parse into json
    let response = reqwest::Client::new()
        .get("https://api.podcastindex.org/api/1.0/value/byepisodeguid")
        .header(USER_AGENT, format!("Helipad/{}", app_version))
        .query(&query)
        .send()
        .await?;

    let result = response.text().await?;
    let json: Value = serde_json::from_str(&result)?;

    let status = json["status"].as_str().unwrap_or_default();

    if status != "true" {
        return Ok(guid); // not found?
    }

    if let Some(query) = json["query"].as_object() {
        guid.podcast_guid = query["podcastguid"].as_str().unwrap_or_default().to_string();
        guid.episode_guid = query["episodeguid"].as_str().unwrap_or_default().to_string();
    }

    if let Some(value) =json["value"].as_object() {
        guid.podcast = Some(value["feedTitle"].as_str().unwrap_or_default().to_string());
        guid.episode = Some(value["title"].as_str().unwrap_or_default().to_string());
    }

    Ok(guid)
}
