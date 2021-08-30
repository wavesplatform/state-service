use super::{Rejection, AppError, ValidationErrorCode, ErrorDetails};
use std::{collections::HashMap};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct HistoricalRequestParams {
    pub block_timestamp: Option<DateTime<Utc>>,
    pub height: Option<i64>,
}

macro_rules! get_parami64 {
    ($i:ident, $e:expr) => {
        match $i.get(&$e.to_string()) {
            Some(s) => {
                match s.parse::<i64>() {
                    Ok(i) => Some(i),
                    Err(e) => {
                        let details = ErrorDetails {
                            parameter: $e.to_string(),
                            reason: format!("{}", e),
                        };

                        return 
                            Err(
                                warp::reject::custom::<AppError>(
                                    AppError::new_validation_error(
                                        ValidationErrorCode::InvalidParamenterValue, details)
                                    )
                            )
                    }
                }
            }
            None => None
        }
    };
}

impl HistoricalRequestParams {
    pub fn from_hashmap(m: &HashMap<String, String>) -> Result<Self, Rejection> {
        let mut block_timestamp: Option<DateTime<Utc>>  = None;

        match m.get("block_timestamp") {
            Some(d) => {
                match DateTime::parse_from_rfc3339(&d) {
                    Ok(d) => block_timestamp = Some(d.into()),
                    Err(e) => {
                        let details = ErrorDetails {
                            parameter: d.clone(),
                            reason: format!("{}", e),
                        };

                        return 
                            Err(
                                warp::reject::custom::<AppError>(
                                    AppError::new_validation_error(
                                        ValidationErrorCode::InvalidParamenterValue, details)
                                    )
                            )
                    }
                }
            },
            None => {}
        }

        let height = get_parami64!(m, "height");
        
        let res = Self {
                block_timestamp: block_timestamp,
                height: height,
        };

        res.check_valid()?;
        Ok(res)
    }

    pub fn is_empty(&self) -> bool {
        self.block_timestamp.is_none() && self.height.is_none()
    }

    pub fn check_valid(&self) -> Result<(), Rejection> {
        if self.block_timestamp.is_some() && self.height.is_some() {
            let details = ErrorDetails {
                parameter: "height, block_timestamp".into(),
                reason: "only one historical parameter must be used".into(),
            };

            return 
                Err(
                    warp::reject::custom::<AppError>(
                        AppError::new_validation_error(
                            ValidationErrorCode::InvalidParamenterValue, details)
                        )
                )
        }
            
        Ok(())
    }
}
