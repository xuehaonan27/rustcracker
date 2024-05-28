use super::RequestError;

/// Wrapper over supported Media Types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaType {
    /// Media Type: "text/plain".
    PlainText,
    /// Media Type: "application/json".
    ApplicationJson,
}

impl Default for MediaType {
    /// Default value for MediaType is application/json
    fn default() -> Self {
        Self::ApplicationJson
    }
}

impl MediaType {
    /// Parses a byte slice into a MediaType structure for a HTTP request. MediaType
    /// must be ASCII, so also UTF-8 valid.
    ///
    /// # Errors
    /// The function returns `InvalidRequest` when parsing the byte stream fails or
    /// unsupported MediaType found.
    ///
    /// # Examples
    ///
    /// ```
    /// use micro_http::MediaType;
    ///
    /// assert!(MediaType::try_from(b"application/json").is_ok());
    /// assert!(MediaType::try_from(b"application/json2").is_err());
    /// ```
    pub fn try_from(bytes: &[u8]) -> Result<Self, RequestError> {
        if bytes.is_empty() {
            return Err(RequestError::InvalidRequest);
        }
        let utf8_slice =
            String::from_utf8(bytes.to_vec()).map_err(|_| RequestError::InvalidRequest)?;
        match utf8_slice.as_str().trim() {
            "text/plain" => Ok(Self::PlainText),
            "application/json" => Ok(Self::ApplicationJson),
            _ => Err(RequestError::InvalidRequest),
        }
    }

    /// Returns a static string representation of the object.
    ///
    /// # Examples
    ///
    /// ```
    /// use micro_http::MediaType;
    ///
    /// let media_type = MediaType::ApplicationJson;
    /// assert_eq!(media_type.as_str(), "application/json");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlainText => "text/plain",
            Self::ApplicationJson => "application/json",
        }
    }
}
