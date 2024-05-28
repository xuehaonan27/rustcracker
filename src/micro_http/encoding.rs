use super::{HttpHeaderError, EncodingError};

/// Wrapper over supported AcceptEncoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoding {}

impl Encoding {
    /// Parses a byte slice and checks if identity encoding is invalidated. Encoding
    /// must be ASCII, so also UTF-8 valid.
    ///
    /// # Errors
    /// `InvalidRequest` is returned when the byte stream is empty.
    ///
    /// `InvalidValue` is returned when the identity encoding is invalidated.
    ///
    /// `InvalidUtf8String` is returned when the byte stream contains invalid characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use micro_http::Encoding;
    ///
    /// assert!(Encoding::try_from(b"deflate").is_ok());
    /// assert!(Encoding::try_from(b"identity;q=0").is_err());
    /// ```
    pub fn try_from(bytes: &[u8]) -> Result<(), EncodingError> {
        if bytes.is_empty() {
            return Err(EncodingError::InvalidValue);
        }
        match std::str::from_utf8(bytes) {
            Ok(headers_str) => {
                let entry = headers_str.split(',').collect::<Vec<&str>>();

                for encoding in entry {
                    match encoding.trim() {
                        "identity;q=0" => {
                            Err(EncodingError::HeaderError(HttpHeaderError::InvalidValue(
                                "Accept-Encoding".to_string(),
                                encoding.to_string(),
                            )))
                        }
                        "*;q=0" if !headers_str.contains("identity") => {
                            Err(EncodingError::HeaderError(HttpHeaderError::InvalidValue(
                                "Accept-Encoding".to_string(),
                                encoding.to_string(),
                            )))
                        }
                        _ => Ok(()),
                    }?;
                }
                Ok(())
            }
            Err(utf8_err) => Err(EncodingError::HeaderError(
                HttpHeaderError::InvalidUtf8String(utf8_err),
            )),
        }
    }
}
