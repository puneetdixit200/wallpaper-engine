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

#[derive(Debug, Deserialize)]
struct PixabaySearchResponse {
    hits: Vec<PixabayImage>,
}

#[derive(Debug, Deserialize)]
struct PixabayImage {
    id: u64,
    #[serde(rename = "webformatURL")]
    webformat_url: String,
    #[serde(rename = "largeImageURL")]
    large_image_url: String,
    #[serde(rename = "imageWidth")]
    image_width: u32,
    #[serde(rename = "imageHeight")]
    image_height: u32,
    user: String,
}

#[derive(Debug, Deserialize)]
struct WallhavenSearchResponse {
    data: Vec<WallhavenImage>,
}

#[derive(Debug, Deserialize)]
struct WallhavenImage {
    id: String,
    path: String,
    category: String,
    purity: String,
    dimension_x: u32,
    dimension_y: u32,
    thumbs: WallhavenThumbs,
}

#[derive(Debug, Deserialize)]
struct WallhavenThumbs {
    large: String,
}

#[derive(Debug, Deserialize)]
struct PicsumImage {
    id: String,
    author: String,
    width: u32,
    height: u32,
    download_url: String,
}

#[derive(Debug, Deserialize)]
struct DeviantArtTagResponse {
    results: Vec<DeviantArtDeviation>,
}

#[derive(Debug, Deserialize)]
struct DeviantArtDeviation {
    deviationid: String,
    is_mature: bool,
    author: DeviantArtAuthor,
    preview: Option<DeviantArtImage>,
    content: Option<DeviantArtImage>,
    thumbs: Option<Vec<DeviantArtImage>>,
}

#[derive(Debug, Deserialize)]
struct DeviantArtAuthor {
    username: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DeviantArtImage {
    src: String,
    height: u32,
    width: u32,
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
        .filter(is_desktop_wallpaper)
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
        .filter(is_desktop_wallpaper)
        .collect()
}

pub fn map_pixabay_search(raw: &str, query: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed = serde_json::from_str::<PixabaySearchResponse>(raw)
        .map_err(|error| format!("Could not parse Pixabay response: {error}"))?;

    Ok(parsed
        .hits
        .into_iter()
        .map(|image| Wallpaper {
            id: format!("pixabay-{}", image.id),
            source: "pixabay".into(),
            thumb_url: image.webformat_url,
            full_url: image.large_image_url,
            photographer: image.user,
            width: image.image_width,
            height: image.image_height,
            query_used: Some(query.to_string()),
            local_path: None,
            is_favorite: false,
        })
        .filter(is_desktop_wallpaper)
        .collect())
}

pub fn map_wallhaven_search(raw: &str, query: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed = serde_json::from_str::<WallhavenSearchResponse>(raw)
        .map_err(|error| format!("Could not parse Wallhaven response: {error}"))?;

    Ok(parsed
        .data
        .into_iter()
        .map(|image| Wallpaper {
            id: format!("wallhaven-{}", image.id),
            source: "wallhaven".into(),
            thumb_url: image.thumbs.large,
            full_url: image.path,
            photographer: format!("Wallhaven {} {}", image.category, image.purity),
            width: image.dimension_x,
            height: image.dimension_y,
            query_used: Some(query.to_string()),
            local_path: None,
            is_favorite: false,
        })
        .filter(is_desktop_wallpaper)
        .collect())
}

pub fn map_picsum_list(raw: &str, query: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed = serde_json::from_str::<Vec<PicsumImage>>(raw)
        .map_err(|error| format!("Could not parse Picsum response: {error}"))?;

    Ok(parsed
        .into_iter()
        .map(|image| Wallpaper {
            id: format!("picsum-{}", image.id),
            source: "picsum".into(),
            thumb_url: format!("https://picsum.photos/id/{}/600/400", image.id),
            full_url: image.download_url,
            photographer: image.author,
            width: image.width,
            height: image.height,
            query_used: Some(query.to_string()),
            local_path: None,
            is_favorite: false,
        })
        .filter(is_desktop_wallpaper)
        .collect())
}

pub fn map_deviantart_tag(raw: &str, query: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed = serde_json::from_str::<DeviantArtTagResponse>(raw)
        .map_err(|error| format!("Could not parse DeviantArt response: {error}"))?;

    Ok(parsed
        .results
        .into_iter()
        .filter(|deviation| !deviation.is_mature)
        .filter_map(|deviation| {
            let full = deviation.content.or_else(|| deviation.preview.clone())?;
            let thumb = deviation
                .preview
                .or_else(|| {
                    deviation
                        .thumbs
                        .and_then(|mut thumbs| thumbs.drain(..).next())
                })
                .unwrap_or_else(|| full.clone());
            Some(Wallpaper {
                id: format!("deviantart-{}", deviation.deviationid),
                source: "deviantart".into(),
                thumb_url: thumb.src,
                full_url: full.src,
                photographer: deviation.author.username,
                width: full.width,
                height: full.height,
                query_used: Some(query.to_string()),
                local_path: None,
                is_favorite: false,
            })
        })
        .filter(is_desktop_wallpaper)
        .collect())
}

fn is_desktop_wallpaper(wallpaper: &Wallpaper) -> bool {
    if wallpaper.width < 1280 || wallpaper.height < 720 || wallpaper.width < wallpaper.height {
        return false;
    }

    let ratio = wallpaper.width as f32 / wallpaper.height as f32;
    (1.3..=5.5).contains(&ratio)
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
        .query(&[
            ("query", query),
            ("per_page", "20"),
            ("page", page.as_str()),
            ("orientation", "landscape"),
        ])
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
        .query(&[
            ("query", query),
            ("per_page", "20"),
            ("page", page.as_str()),
            ("orientation", "landscape"),
        ])
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

pub async fn fetch_pixabay(
    client: &Client,
    query: &str,
    page: u32,
    key: &str,
) -> Result<Vec<Wallpaper>, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Pixabay API key is missing. Add it in Settings.".into());
    }

    let page = page.max(1).to_string();
    let response = client
        .get("https://pixabay.com/api/")
        .query(&[
            ("key", key),
            ("q", query),
            ("image_type", "photo"),
            ("orientation", "horizontal"),
            ("safesearch", "true"),
            ("per_page", "20"),
            ("page", page.as_str()),
        ])
        .send()
        .await
        .map_err(|error| format!("Pixabay request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Pixabay response: {error}"))?;
    if !status.is_success() {
        return Err(format!("Pixabay returned {status}: {body}"));
    }

    map_pixabay_search(&body, query)
}

pub async fn fetch_wallhaven(
    client: &Client,
    query: &str,
    page: u32,
    key: &str,
    random: bool,
    allow_nsfw: bool,
) -> Result<Vec<Wallpaper>, String> {
    let page = page.max(1).to_string();
    let purity = wallhaven_purity(allow_nsfw, key)?;
    let mut request = client.get("https://wallhaven.cc/api/v1/search").query(&[
        ("categories", "111"),
        ("purity", purity),
        ("atleast", "1920x1080"),
        ("ratios", "16x9,16x10,21x9,32x9,48x9"),
        ("page", page.as_str()),
        ("sorting", if random { "random" } else { "relevance" }),
        ("order", "desc"),
    ]);

    if !query.trim().is_empty() {
        request = request.query(&[("q", query.trim())]);
    }
    if !key.trim().is_empty() {
        request = request.query(&[("apikey", key.trim())]);
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("Wallhaven request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Wallhaven response: {error}"))?;
    if !status.is_success() {
        return Err(format!("Wallhaven returned {status}: {body}"));
    }

    map_wallhaven_search(
        &body,
        if query.trim().is_empty() {
            "wallpaper"
        } else {
            query
        },
    )
}

pub fn wallhaven_purity(allow_nsfw: bool, key: &str) -> Result<&'static str, String> {
    if !allow_nsfw {
        return Ok("100");
    }

    if key.trim().is_empty() {
        return Err("Wallhaven NSFW requires a Wallhaven API key. Add it in Settings.".into());
    }

    Ok("111")
}

pub async fn fetch_picsum(
    client: &Client,
    query: &str,
    page: u32,
) -> Result<Vec<Wallpaper>, String> {
    let page = page.max(1).to_string();
    let response = client
        .get("https://picsum.photos/v2/list")
        .query(&[("page", page.as_str()), ("limit", "20")])
        .send()
        .await
        .map_err(|error| format!("Picsum request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Picsum response: {error}"))?;
    if !status.is_success() {
        return Err(format!("Picsum returned {status}: {body}"));
    }

    map_picsum_list(&body, query)
}

pub async fn fetch_deviantart(
    client: &Client,
    query: &str,
    page: u32,
    token: &str,
) -> Result<Vec<Wallpaper>, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("DeviantArt access token is missing. Add it in Settings.".into());
    }

    let tag = query
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("wallpaper");
    let offset = ((page.max(1) - 1) * 20).to_string();
    let response = client
        .get("https://www.deviantart.com/api/v1/oauth2/browse/tags")
        .query(&[
            ("tag", tag),
            ("limit", "20"),
            ("offset", offset.as_str()),
            ("mature_content", "false"),
        ])
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| format!("DeviantArt request failed: {error}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read DeviantArt response: {error}"))?;
    if !status.is_success() {
        return Err(format!("DeviantArt returned {status}: {body}"));
    }

    map_deviantart_tag(&body, query)
}

pub fn fetch_artstation_unsupported() -> Result<Vec<Wallpaper>, String> {
    Err("ArtStation does not provide a stable public search API. Unofficial scraping is not enabled.".into())
}

pub async fn search_wallpapers(
    client: &Client,
    query: &str,
    page: u32,
    source: ApiSource,
    keys: &ApiKeys,
    allow_nsfw_wallhaven: bool,
) -> Result<Vec<Wallpaper>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Type something to search for wallpapers.".into());
    }

    match source {
        ApiSource::Pexels => fetch_pexels(client, query, page, &keys.pexels).await,
        ApiSource::Unsplash => fetch_unsplash(client, query, page, &keys.unsplash).await,
        ApiSource::Pixabay => fetch_pixabay(client, query, page, &keys.pixabay).await,
        ApiSource::Wallhaven => {
            fetch_wallhaven(
                client,
                query,
                page,
                &keys.wallhaven,
                false,
                allow_nsfw_wallhaven,
            )
            .await
        }
        ApiSource::Picsum => fetch_picsum(client, query, page).await,
        ApiSource::DeviantArt => fetch_deviantart(client, query, page, &keys.deviantart).await,
        ApiSource::ArtStation => fetch_artstation_unsupported(),
        ApiSource::All => {
            let mut results = Vec::new();
            if !keys.pexels.trim().is_empty() {
                results.push(fetch_pexels(client, query, page, &keys.pexels).await);
            }
            if !keys.unsplash.trim().is_empty() {
                results.push(fetch_unsplash(client, query, page, &keys.unsplash).await);
            }
            if !keys.pixabay.trim().is_empty() {
                results.push(fetch_pixabay(client, query, page, &keys.pixabay).await);
            }
            results.push(
                fetch_wallhaven(
                    client,
                    query,
                    page,
                    &keys.wallhaven,
                    false,
                    allow_nsfw_wallhaven,
                )
                .await,
            );
            results.push(fetch_picsum(client, query, page).await);
            if !keys.deviantart.trim().is_empty() {
                results.push(fetch_deviantart(client, query, page, &keys.deviantart).await);
            }
            merge_results(results)
        }
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
    allow_nsfw_wallhaven: bool,
) -> Result<Vec<Wallpaper>, String> {
    match source {
        ApiSource::Pexels => fetch_pexels_curated(client, &keys.pexels).await,
        ApiSource::Unsplash => fetch_unsplash_random(client, &keys.unsplash).await,
        ApiSource::Pixabay => fetch_pixabay(client, "wallpaper", 1, &keys.pixabay).await,
        ApiSource::Wallhaven => {
            fetch_wallhaven(client, "", 1, &keys.wallhaven, true, allow_nsfw_wallhaven).await
        }
        ApiSource::Picsum => fetch_picsum(client, "random", 1).await,
        ApiSource::DeviantArt => fetch_deviantart(client, "wallpaper", 1, &keys.deviantart).await,
        ApiSource::ArtStation => fetch_artstation_unsupported(),
        ApiSource::All => {
            let mut results = Vec::new();
            if !keys.pexels.trim().is_empty() {
                results.push(fetch_pexels_curated(client, &keys.pexels).await);
            }
            if !keys.unsplash.trim().is_empty() {
                results.push(fetch_unsplash_random(client, &keys.unsplash).await);
            }
            if !keys.pixabay.trim().is_empty() {
                results.push(fetch_pixabay(client, "wallpaper", 1, &keys.pixabay).await);
            }
            results.push(
                fetch_wallhaven(client, "", 1, &keys.wallhaven, true, allow_nsfw_wallhaven).await,
            );
            results.push(fetch_picsum(client, "random", 1).await);
            if !keys.deviantart.trim().is_empty() {
                results.push(fetch_deviantart(client, "wallpaper", 1, &keys.deviantart).await);
            }
            merge_results(results)
        }
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
        assert_eq!(
            wallpapers[0].thumb_url,
            "https://images.pexels.com/thumb.jpg"
        );
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
        assert_eq!(
            wallpapers[0].thumb_url,
            "https://images.unsplash.com/thumb.jpg"
        );
        assert_eq!(
            wallpapers[0].full_url,
            "https://images.unsplash.com/full.jpg"
        );
        assert_eq!(wallpapers[0].photographer, "Unsplash Person");
        assert_eq!(wallpapers[0].query_used.as_deref(), Some("city"));
    }

    #[test]
    fn maps_pixabay_search_response_to_wallpaper_model() {
        let raw = r#"{
          "hits": [
            {
              "id": 195893,
              "webformatURL": "https://pixabay.com/thumb.jpg",
              "largeImageURL": "https://pixabay.com/large.jpg",
              "imageWidth": 4000,
              "imageHeight": 2250,
              "user": "Josch13"
            }
          ]
        }"#;

        let wallpapers =
            map_pixabay_search(raw, "yellow flowers").expect("pixabay response should map");

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].id, "pixabay-195893");
        assert_eq!(wallpapers[0].source, "pixabay");
        assert_eq!(wallpapers[0].thumb_url, "https://pixabay.com/thumb.jpg");
        assert_eq!(wallpapers[0].full_url, "https://pixabay.com/large.jpg");
        assert_eq!(wallpapers[0].photographer, "Josch13");
        assert_eq!(wallpapers[0].query_used.as_deref(), Some("yellow flowers"));
    }

    #[test]
    fn filters_non_desktop_sized_provider_results() {
        let raw = r#"{
          "hits": [
            {
              "id": 195893,
              "webformatURL": "https://pixabay.com/desktop-thumb.jpg",
              "largeImageURL": "https://pixabay.com/desktop-large.jpg",
              "imageWidth": 4000,
              "imageHeight": 2250,
              "user": "Desktop"
            },
            {
              "id": 195894,
              "webformatURL": "https://pixabay.com/phone-thumb.jpg",
              "largeImageURL": "https://pixabay.com/phone-large.jpg",
              "imageWidth": 1080,
              "imageHeight": 1920,
              "user": "Phone"
            },
            {
              "id": 195895,
              "webformatURL": "https://pixabay.com/small-thumb.jpg",
              "largeImageURL": "https://pixabay.com/small-large.jpg",
              "imageWidth": 1024,
              "imageHeight": 768,
              "user": "Small"
            }
          ]
        }"#;

        let wallpapers = map_pixabay_search(raw, "wallpaper").expect("pixabay response should map");

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].id, "pixabay-195893");
    }

    #[test]
    fn maps_wallhaven_search_response_to_wallpaper_model() {
        let raw = r#"{
          "data": [
            {
              "id": "94x38z",
              "path": "https://w.wallhaven.cc/full/94/wallhaven-94x38z.jpg",
              "category": "anime",
              "purity": "sfw",
              "dimension_x": 6742,
              "dimension_y": 3534,
              "thumbs": {
                "large": "https://th.wallhaven.cc/lg/94/94x38z.jpg"
              }
            }
          ]
        }"#;

        let wallpapers = map_wallhaven_search(raw, "anime").expect("wallhaven response should map");

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].id, "wallhaven-94x38z");
        assert_eq!(wallpapers[0].source, "wallhaven");
        assert_eq!(
            wallpapers[0].thumb_url,
            "https://th.wallhaven.cc/lg/94/94x38z.jpg"
        );
        assert_eq!(
            wallpapers[0].full_url,
            "https://w.wallhaven.cc/full/94/wallhaven-94x38z.jpg"
        );
        assert_eq!(wallpapers[0].photographer, "Wallhaven anime sfw");
        assert_eq!(wallpapers[0].width, 6742);
        assert_eq!(wallpapers[0].height, 3534);
    }

    #[test]
    fn wallhaven_purity_stays_sfw_until_nsfw_is_allowed_with_key() {
        assert_eq!(
            wallhaven_purity(false, "").expect("sfw should not require a key"),
            "100"
        );
        assert!(wallhaven_purity(true, "")
            .expect_err("nsfw should require an api key")
            .contains("Wallhaven NSFW requires"));
        assert_eq!(
            wallhaven_purity(true, "wallhaven-key").expect("key should allow nsfw"),
            "111"
        );
    }

    #[test]
    fn maps_picsum_list_response_to_wallpaper_model() {
        let raw = r#"[
          {
            "id": "0",
            "author": "Alejandro Escamilla",
            "width": 5616,
            "height": 3744,
            "download_url": "https://picsum.photos/id/0/5616/3744"
          }
        ]"#;

        let wallpapers = map_picsum_list(raw, "random").expect("picsum response should map");

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].id, "picsum-0");
        assert_eq!(wallpapers[0].source, "picsum");
        assert_eq!(
            wallpapers[0].thumb_url,
            "https://picsum.photos/id/0/600/400"
        );
        assert_eq!(
            wallpapers[0].full_url,
            "https://picsum.photos/id/0/5616/3744"
        );
        assert_eq!(wallpapers[0].photographer, "Alejandro Escamilla");
    }

    #[test]
    fn maps_deviantart_tag_response_to_wallpaper_model() {
        let raw = r#"{
          "results": [
            {
              "deviationid": "75825C66-FF9B-9AB8-2C06-3A0ED19ED58B",
              "is_mature": false,
              "author": { "username": "lonefirewarrior" },
              "preview": {
                "src": "https://th08.deviantart.net/preview.jpg",
                "height": 670,
                "width": 1192
              },
              "content": {
                "src": "https://fc05.deviantart.net/full.jpg",
                "height": 1080,
                "width": 1920
              }
            }
          ]
        }"#;

        let wallpapers = map_deviantart_tag(raw, "nature").expect("deviantart response should map");

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(
            wallpapers[0].id,
            "deviantart-75825C66-FF9B-9AB8-2C06-3A0ED19ED58B"
        );
        assert_eq!(wallpapers[0].source, "deviantart");
        assert_eq!(
            wallpapers[0].thumb_url,
            "https://th08.deviantart.net/preview.jpg"
        );
        assert_eq!(
            wallpapers[0].full_url,
            "https://fc05.deviantart.net/full.jpg"
        );
        assert_eq!(wallpapers[0].photographer, "lonefirewarrior");
    }
}
