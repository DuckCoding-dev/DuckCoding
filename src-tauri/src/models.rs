use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct NpmPackageInfo {
    #[serde(rename = "dist-tags")]
    pub dist_tags: NpmDistTags,
}

#[derive(Deserialize, Debug)]
pub struct NpmDistTags {
    pub latest: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct NodeEnvironment {
    pub node_available: bool,
    pub node_version: Option<String>,
    pub npm_available: bool,
    pub npm_version: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub output: String,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub has_update: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ActiveConfig {
    pub api_key: String,
    pub base_url: String,
    pub profile_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    pub user_id: String,
    pub system_token: String,
}

#[derive(Deserialize, Debug)]
pub struct TokenData {
    pub id: i64,
    pub key: String,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub group: String,
}

#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<TokenData>>,
}

#[derive(Serialize)]
pub struct GenerateApiKeyResult {
    pub success: bool,
    pub message: String,
    pub api_key: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UsageData {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub model_name: String,
    pub created_at: i64,
    pub token_used: i64,
    pub count: i64,
    pub quota: i64,
}

#[derive(Deserialize, Debug)]
pub struct UsageApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<UsageData>>,
}

#[derive(Serialize)]
pub struct UsageStatsResult {
    pub success: bool,
    pub message: String,
    pub data: Vec<UsageData>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub quota: i64,
    pub used_quota: i64,
    pub request_count: i64,
}

#[derive(Deserialize, Debug)]
pub struct UserApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<UserInfo>,
}

#[derive(Serialize)]
pub struct UserQuotaResult {
    pub success: bool,
    pub message: String,
    pub total_quota: f64,
    pub used_quota: f64,
    pub remaining_quota: f64,
    pub request_count: i64,
}
