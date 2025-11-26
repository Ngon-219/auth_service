use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct DateRangeQuery {
    /// Start date in format YYYY-MM-DD (inclusive)
    pub start_date: String,
    /// End date in format YYYY-MM-DD (inclusive)
    pub end_date: String,
}

impl DateRangeQuery {
    pub fn to_range(&self) -> Result<(NaiveDateTime, NaiveDateTime), String> {
        let start = NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d")
            .map_err(|e| format!("Invalid start_date format: {}", e))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| "Invalid start_date time".to_string())?;

        let end = NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d")
            .map_err(|e| format!("Invalid end_date format: {}", e))?
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| "Invalid end_date time".to_string())?;

        Ok((start, end))
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TimeSeriesPoint {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserStatsResponse {
    pub total_users: i64,
    pub total_students: i64,
    pub total_managers: i64,
    pub total_teachers: i64,
    pub total_admins: i64,
    pub users_per_day: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentStatsResponse {
    pub total_documents: i64,
    pub signed_documents: i64,
    pub failed_documents: i64,
    pub documents_per_day: Vec<TimeSeriesPoint>,
}


