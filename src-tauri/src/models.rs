use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ApiSource {
    All,
    Pexels,
    Unsplash,
    Pixabay,
    Wallhaven,
    Picsum,
    DeviantArt,
    ArtStation,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Wallpaper {
    pub id: String,
    pub source: String,
    pub thumb_url: String,
    pub full_url: String,
    pub photographer: String,
    pub width: u32,
    pub height: u32,
    pub query_used: Option<String>,
    pub local_path: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub favorites: Vec<Wallpaper>,
    pub downloaded: Vec<Wallpaper>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub bytes: u64,
    pub files: u64,
}
