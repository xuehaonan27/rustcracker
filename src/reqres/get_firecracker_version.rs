use either::Either;

use crate::{
    agent::{serialize_request, HttpRequest},
    models::{FirecrackerVersion, InternalError},
};

use super::*;

#[repr(transparent)]
pub struct GetFirecrackerVersionRequest;

impl FirecrackerRequest for GetFirecrackerVersionRequest {
    fn to_string(&self) -> String {
        let request = HttpRequest::new("GET", "/version", None, None);
        serialize_request(&request)
    }
}

#[repr(transparent)]
pub struct GetFirecrackerVersionResponse(pub Either<FirecrackerVersion, InternalError>);

impl FirecrackerResponse for GetFirecrackerVersionResponse {
    type Payload = Either<FirecrackerVersion, InternalError>;

    #[inline]
    fn is_succ(&self) -> bool {
        self.0.is_left()
    }

    #[inline]
    fn is_err(&self) -> bool {
        self.0.is_right()
    }

    fn decode(payload: &Option<Vec<u8>>) -> crate::RtckResult<Self>
    where
        Self: Sized,
    {
        if let Some(payload) = payload {
            match serde_json::from_slice::<FirecrackerVersion>(&payload) {
                Ok(content) => Ok(Self(either::Left(content))),
                Err(_) => match serde_json::from_slice::<InternalError>(&payload) {
                    Ok(content) => Ok(Self(either::Right(content))),
                    Err(e) => Err(crate::RtckError::Decode(e.to_string())),
                },
            }
        } else {
            Err(crate::RtckError::Decode(
                "error type, expecting FirecrackerVersion".into(),
            ))
        }
    }
}

pub struct GetFirecrackerVersion(pub GetFirecrackerVersionRequest);

impl GetFirecrackerVersion {
    pub fn new() -> Self {
        Self(GetFirecrackerVersionRequest)
    }
}

impl FirecrackerEvent for GetFirecrackerVersion {
    type Req = GetFirecrackerVersionRequest;
    type Res = GetFirecrackerVersionResponse;

    fn req(&self) -> String {
        self.0.to_string()
    }

    fn decode(payload: &Option<Vec<u8>>) -> RtckResult<Self::Res> {
        Self::Res::decode(payload)
    }
}
