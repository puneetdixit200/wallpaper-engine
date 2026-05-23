use crate::models::{ApiSource, Wallpaper};
use crate::settings::ApiKeys;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PexelsSearchResponse {
    photos: Vec<PexelsPhoto>,
}

#[derive(Debug, Deserialize)]
struct PexelsPhoto {
    id: u64,
    width: u32,
    height: u32,
    photographer: String,
    src: PexelsSources,
}

#[derive(Debug, Deserialize)]
struct PexelsSources {
    original: String,
    large2x: Option<String>,
    medium: String,
}

#[derive(Debug, Deserialize)]
struct UnsplashSearchResponse {
    results: Vec<UnsplashPhoto>,
}

#[derive(Debug, Deserialize)]
struct UnsplashPhoto {
    id: String,
    width: u32,
    height: u32,
    urls: UnsplashUrls,
    user: UnsplashUser,
}

#[derive(Debug, Deserialize)]
struct UnsplashUrls {
    full: String,
    regular: String,
    thumb: String,
}

#[derive(Debug, Deserialize)]
struct UnsplashUser {
    name: String,
}

pub fn map_pexels_search(raw: &str, query: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed = serde_json::from_str::<PexelsSearchResponse>(raw)
        .map_err(|error| format!("Could not parse Pexels response: {error}"))?;

    Ok(parsed
        .photos
        .into_iter()
        .map(|photo| Wallpaper {
            id: format!("pexels-{}", photo.id),
            source: "pexels".into(),
            thumb_url: photo.src.medium,
            full_url: if photo.src.original.is_empty() {
                photo.src.large2x.unwrap_or_default()
            } else {
                photo.src.original
            },
            photographer: photo.photographer,
            width: photo.width,
            height: photo.height,
            query_used: Some(query.to_string()),
            local_path: None,
            is_favorite: false,
        })
        .collect())
}

pub fn map_unsplash_search(raw: &str, query: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed = serde_json::from_str::<UnsplashSearchResponse>(raw)
        .map_err(|error| format!("Could not parse Unsplash response: {error}"))?;
    Ok(map_unsplash_photos(parsed.results, query))
}

pub fn map_unsplash_random(raw: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed = serde_json::from_str::<Vec<UnsplashPhoto>>(raw)
        .map_err(|error| format!("Could not parse Unsplash random response: {error}"))?;
    Ok(map_unsplash_photos(parsed, "random"))
}

fn map_unsplash_photos(photos: Vec<UnsplashPhoto>, query: &str) -> Vec<Wallpaper> {
    photos
        .into_iter()
        .map(|photo| Wallpaper {
            id: format!("unsplash-{}", photo.id),
            source: "unsplash".into(),
            thumb_url: photo.urls.thumb,
            full_url: if photo.urls.full.is_empty() {
                photo.urls.regular
            } else {
                photo.urls.full
            },
            photographer: photo.user.name,
            width: photo.width,
            height: photo.height,
            query_used: Some(query.to_string()),
            local_path: None,
            is_favorite: false,
        })
        .collect()
}

pub async fn fetch_pexels(
    client: &Client,
    query: &str,
    page: u32,
    key: &str,
) -> Result<Vec<Wallpaper>, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Pexels API key is missing. Add it in Settings.".into());
    }

    let page = page.max(1).to_string();
    let response = client
        .get("https://api.pexels.com/v1/search")
        .query(&[("query", query), ("per_page", "20"), ("page", page.as_str())])
        .header("Authorization", key)
        .send()
        .await
        .map_err(|error| format!("Pexels request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Pexels response: {error}"))?;
    if !status.is_success() {
        return Err(format!("Pexels returned {status}: {body}"));
    }

    map_pexels_search(&body, query)
}

pub async fn fetch_pexels_curated(client: &Client, key: &str) -> Result<Vec<Wallpaper>, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Pexels API key is missing. Add it in Settings.".into());
    }

    let response = client
        .get("https://api.pexels.com/v1/curated")
        .query(&[("per_page", "20")])
        .header("Authorization", key)
        .send()
        .await
        .map_err(|error| format!("Pexels curated request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Pexels curated response: {error}"))?;
    if !status.is_success() {
        return Err(format!("Pexels returned {status}: {body}"));
    }

    map_pexels_search(&body, "curated")
}

pub async fn fetch_unsplash(
    client: &Client,
    query: &str,
    page: u32,
    key: &str,
) -> Result<Vec<Wallpaper>, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Unsplash API key is missing. Add it in Settings.".into());
    }

    let page = page.max(1).to_string();
    let response = client
        .get("https://api.unsplash.com/search/photos")
        .query(&[("query", query), ("per_page", "20"), ("page", page.as_str())])
        .header("Authorization", format!("Client-ID {key}"))
        .send()
        .await
        .map_err(|error| format!("Unsplash request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Unsplash response: {error}"))?;
    if !status.is_success() {
        return Err(format!("Unsplash returned {status}: {body}"));
    }

    map_unsplash_search(&body, query)
}

pub async fn fetch_unsplash_random(client: &Client, key: &str) -> Result<Vec<Wallpaper>, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Unsplash API key is missing. Add it in Settings.".into());
    }

    let response = client
        .get("https://api.unsplash.com/photos/random")
        .query(&[("count", "20"), ("orientation", "landscape")])
        .header("Authorization", format!("Client-ID {key}"))
        .send()
        .await
        .map_err(|error| format!("Unsplash random request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Unsplash random response: {error}"))?;
    if !status.is_success() {
        return Err(format!("Unsplash returned {status}: {body}"));
    }

    map_unsplash_random(&body)
}

pub async fn search_wallpapers(
    client: &Client,
    query: &str,
    page: u32,
    source: ApiSource,
    keys: &ApiKeys,
) -> Result<Vec<Wallpaper>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Type something to search for wallpapers.".into());
    }

    match source {
        ApiSource::Pexels => fetch_pexels(client, query, page, &keys.pexels).await,
        ApiSource::Unsplash => fetch_unsplash(client, query, page, &keys.unsplash).await,
        ApiSource::Both => {
            fetch_from_both(
                fetch_pexels(client, query, page, &keys.pexels),
                fetch_unsplash(client, query, page, &keys.unsplash),
                !keys.pexels.trim().is_empty(),
                !keys.unsplash.trim().is_empty(),
            )
            .await
        }
    }
}

pub async fn random_wallpapers(
    client: &Client,
    source: ApiSource,
    keys: &ApiKeys,
) -> Result<Vec<Wallpaper>, String> {
    match source {
        ApiSource::Pexels => fetch_pexels_curated(client, &keys.pexels).await,
        ApiSource::Unsplash => fetch_unsplash_random(client, &keys.unsplash).await,
        ApiSource::Both => {
            fetch_from_both(
                fetch_pexels_curated(client, &keys.pexels),
                fetch_unsplash_random(client, &keys.unsplash),
                !keys.pexels.trim().is_empty(),
                !keys.unsplash.trim().is_empty(),
            )
            .await
        }
    }
}

async fn fetch_from_both<P, U>(
    pexels_future: P,
    unsplash_future: U,
    has_pexels_key: bool,
    has_unsplash_key: bool,
) -> Result<Vec<Wallpaper>, String>
where
    P: std::future::Future<Output = Result<Vec<Wallpaper>, String>>,
    U: std::future::Future<Output = Result<Vec<Wallpaper>, String>>,
{
    match (has_pexels_key, has_unsplash_key) {
        (true, true) => {
            let (pexels, unsplash) = tokio::join!(pexels_future, unsplash_future);
            merge_results(vec![pexels, unsplash])
        }
        (true, false) => pexels_future.await,
        (false, true) => unsplash_future.await,
        (false, false) => Err("Add a Pexels or Unsplash API key in Settings.".into()),
    }
}

fn merge_results(results: Vec<Result<Vec<Wallpaper>, String>>) -> Result<Vec<Wallpaper>, String> {
    let mut merged = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok(mut wallpapers) => merged.append(&mut wallpapers),
            Err(error) => errors.push(error),
        }
    }

    if merged.is_empty() {
        Err(errors.join("; "))
    } else {
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_pexels_search_response_to_wallpaper_model() {
        let raw = r#"{
          "photos": [
            {
              "id": 42,
              "width": 3840,
              "height": 2160,
              "photographer": "Photo Person",
              "src": {
                "original": "https://images.pexels.com/full.jpg",
                "large2x": "https://images.pexels.com/large.jpg",
                "medium": "https://images.pexels.com/thumb.jpg"
              }
            }
          ]
        }"#;

        let wallpapers = map_pexels_search(raw, "forest").expect("pexels response should map");

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].id, "pexels-42");
        assert_eq!(wallpapers[0].source, "pexels");
        assert_eq!(wallpapers[0].thumb_url, "https://images.pexels.com/thumb.jpg");
        assert_eq!(wallpapers[0].full_url, "https://images.pexels.com/full.jpg");
        assert_eq!(wallpapers[0].photographer, "Photo Person");
        assert_eq!(wallpapers[0].query_used.as_deref(), Some("forest"));
    }

    #[test]
    fn maps_unsplash_search_response_to_wallpaper_model() {
        let raw = r#"{
          "results": [
            {
              "id": "abc123",
              "width": 3000,
              "height": 2000,
              "urls": {
                "full": "https://images.unsplash.com/full.jpg",
                "regular": "https://images.unsplash.com/regular.jpg",
                "thumb": "https://images.unsplash.com/thumb.jpg"
              },
              "user": {
                "name": "Unsplash Person"
              }
            }
          ]
        }"#;

        let wallpapers = map_unsplash_search(raw, "city").expect("unsplash response should map");

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].id, "unsplash-abc123");
        assert_eq!(wallpapers[0].source, "unsplash");
        assert_eq!(wallpapers[0].thumb_url, "https://images.unsplash.com/thumb.jpg");
        assert_eq!(wallpapers[0].full_url, "https://images.unsplash.com/full.jpg");
        assert_eq!(wallpapers[0].photographer, "Unsplash Person");
        assert_eq!(wallpapers[0].query_used.as_deref(), Some("city"));
    }
}
