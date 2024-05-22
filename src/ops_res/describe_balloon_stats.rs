use either::Either;

use crate::{command::Command, models::{balloon_stats::BalloonStatistics, error::InternalError}, ser::Empty};

use super::{Operation, Response};

pub struct DescribeBalloonStatsOps {
    data: Empty,
}

impl DescribeBalloonStatsOps {
    pub fn new() -> Self {
        Self { data: Empty {  } }
    }
}

impl Operation for DescribeBalloonStatsOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::GET,
            url: "/balloon/statistics".into(),
            data: Box::new(self.data)
        }
    }
}

pub struct DescribeBalloonStatsRes {
    data: Either<BalloonStatistics, InternalError>
}

impl DescribeBalloonStatsRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> BalloonStatistics {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for DescribeBalloonStatsRes {
    type Data = Self;
    fn decode(res: &crate::micro_http::HttpResponse) -> crate::RtckResult<Self::Data> {
        if res.is_fine() {
            Ok(Self {
                data: either::Left(serde_json::from_slice(res.body().as_bytes())?),
            })
        } else {
            Ok(Self {
                data: either::Right(serde_json::from_slice(res.body().as_bytes())?),
            })
        }
    }
}