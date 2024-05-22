use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, partial_network_interface::PartialNetworkInterface},
    ser::Empty,
};

use super::{Operation, Response};

pub struct PatchGuestNetworkInterfaceByIdOps {
    data: PartialNetworkInterface,
}

impl PatchGuestNetworkInterfaceByIdOps {
    pub fn new(data: PartialNetworkInterface) -> Self {
        Self { data }
    }
}

impl Operation for PatchGuestNetworkInterfaceByIdOps {
    fn encode(&self) -> crate::command::Command {
        let iface_id = &self.data.iface_id;
        Command {
            method: crate::micro_http::HttpMethod::PATCH,
            url: format!("/network-interfaces/{iface_id}"),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PatchGuestNetworkInterfaceByIdRes {
    data: Either<Empty, InternalError>,
}

impl PatchGuestNetworkInterfaceByIdRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> Empty {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for PatchGuestNetworkInterfaceByIdRes {
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
