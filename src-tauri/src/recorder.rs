pub mod bilibili;
pub mod douyin;
pub mod danmu;
pub mod errors;

mod entry;

use async_trait::async_trait;
use danmu::DanmuEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformType {
    BiliBili,
    Douyin,
    Huya,
    Youtube,
}

impl PlatformType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformType::BiliBili => "bilibili",
            PlatformType::Douyin => "douyin",
            PlatformType::Huya => "huya",
            PlatformType::Youtube => "youtube",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bilibili" => Some(PlatformType::BiliBili),
            "douyin" => Some(PlatformType::Douyin),
            "huya" => Some(PlatformType::Huya),
            "youtube" => Some(PlatformType::Youtube),
            _ => None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct RecorderInfo {
    pub room_id: u64,
    pub room_info: RoomInfo,
    pub user_info: UserInfo,
    pub total_length: f64,
    pub current_live_id: String,
    pub live_status: bool,
    pub platform: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct RoomInfo {
    pub room_id: u64,
    pub room_title: String,
    pub room_cover: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct UserInfo {
    pub user_id: String,
    pub user_name: String,
    pub user_avatar: String,
}

#[async_trait]
pub trait Recorder: Send + Sync + 'static {
    async fn run(&self);
    async fn stop(&self);
    async fn clip_range(&self, live_id: &str, x: f64, y: f64, output_path: &str) -> Result<String, errors::RecorderError>;
    async fn m3u8_content(&self, live_id: &str) -> String;
    async fn info(&self) -> RecorderInfo;
    async fn comments(&self, live_id: &str) -> Result<Vec<DanmuEntry>, errors::RecorderError>;
}