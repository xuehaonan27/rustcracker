use super::error::MethodError;

/// Supported HTTP Methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Method {
    /// GET Method.
    Get,
    /// PUT Method.
    Put,
    /// PATCH Method.
    Patch,
}

impl Method {
    /// Returns a `Method` object if the parsing of `bytes` is successful.
    ///
    /// The method is case sensitive. A call to try_from with the input b"get" will return
    /// an error, but when using the input b"GET", it returns Method::Get.
    ///
    /// # Errors
    /// `InvalidHttpMethod` is returned if the specified HTTP method is unsupported.
    pub fn try_from(bytes: &[u8]) -> Result<Self, MethodError> {
        match bytes {
            b"GET" => Ok(Self::Get),
            b"PUT" => Ok(Self::Put),
            b"PATCH" => Ok(Self::Patch),
            _ => Err(MethodError::InvalidHttpMethod("Unsupported HTTP method.")),
        }
    }

    /// Returns an `u8 slice` corresponding to the Method.
    pub fn raw(self) -> &'static [u8] {
        match self {
            Self::Get => b"GET",
            Self::Put => b"PUT",
            Self::Patch => b"PATCH",
        }
    }

    /// Returns an &str corresponding to the Method.
    pub fn to_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
        }
    }
}
