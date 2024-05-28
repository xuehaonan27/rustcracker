use std::io::{Error as WriteError, Write};

use crate::micro_http::{ascii::CRLF_LEN, find};

use super::{
    ascii::{COLON, CR, LF, SP},
    header::ResponseHeaders,
    Body, Header, MediaType, Method, ResponseError, Version,
};

type ResponseLineParts<'a> = (&'a [u8], &'a [u8], &'a [u8]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusCode {
    /// 100, Continue
    Continue,
    /// 200, OK
    OK,
    /// 204, No Content
    NoContent,
    /// 400, Bad Request
    BadRequest,
    /// 401, Unauthorized
    Unauthorized,
    /// 404, Not Found
    NotFound,
    /// 405, Method Not Allowed
    MethodNotAllowed,
    /// 413, Payload Too Large
    PayloadTooLarge,
    /// 500, Internal Server Error
    InternalServerError,
    /// 501, Not Implemented
    NotImplemented,
    /// 503, Service Unavailable
    ServiceUnavailable,
}

impl StatusCode {
    /// Returns the status code as bytes.
    pub fn raw(self) -> &'static [u8; 3] {
        match self {
            Self::Continue => b"100",
            Self::OK => b"200",
            Self::NoContent => b"204",
            Self::BadRequest => b"400",
            Self::Unauthorized => b"401",
            Self::NotFound => b"404",
            Self::MethodNotAllowed => b"405",
            Self::PayloadTooLarge => b"413",
            Self::InternalServerError => b"500",
            Self::NotImplemented => b"501",
            Self::ServiceUnavailable => b"503",
        }
    }

    fn try_from(code: &[u8], text: &[u8]) -> Result<Self, ResponseError> {
        let code = std::str::from_utf8(code)
            .map_err(|_| ResponseError::InvalidStatusCode("Cannot parse Status Code as UTF-8"))?;
        let text = std::str::from_utf8(text)
            .map_err(|_| ResponseError::InvalidStatusCode("Cannot parse Status Text as UTF-8"))?;
        match (code, text) {
            ("100", "Continue") => Ok(Self::Continue),
            ("200", "OK") => Ok(Self::OK),
            ("204", "No Content") => Ok(Self::NoContent),
            ("400", "Bad Request") => Ok(Self::BadRequest),
            ("401", "Unauthorized") => Ok(Self::Unauthorized),
            ("404", "Not Found") => Ok(Self::NotFound),
            ("405", "Method Not Allowed") => Ok(Self::MethodNotAllowed),
            ("413", "Payload Too Large") => Ok(Self::PayloadTooLarge),
            ("500", "Internal Server Error") => Ok(Self::InternalServerError),
            ("501", "Not Implemented") => Ok(Self::NotImplemented),
            ("503", "Service Unavailable") => Ok(Self::ServiceUnavailable),
            _ => Err(ResponseError::InvalidResponse),
        }
    }
}

#[derive(Debug, PartialEq)]
struct StatusLine {
    http_version: Version,
    status_code: StatusCode,
}

impl StatusLine {
    fn new(http_version: Version, status_code: StatusCode) -> Self {
        Self {
            http_version,
            status_code,
        }
    }

    fn write_all<T: Write>(&self, mut buf: T) -> Result<(), WriteError> {
        buf.write_all(self.http_version.raw())?;
        buf.write_all(&[SP])?;
        buf.write_all(self.status_code.raw())?;
        buf.write_all(&[SP, CR, LF])?;

        Ok(())
    }

    fn parse_status_line(response_line: &[u8]) -> Result<ResponseLineParts, ResponseError> {
        if let Some(version_end) = find(response_line, &[SP]) {
            // The slice access is safe because `find` validates that `version_end` < `response_line` size.
            let version = &response_line[..version_end];

            // `status_start` <= `response_line` size.
            let status_start = version_end.checked_add(1).ok_or(ResponseError::Overflow)?;

            // Slice access is safe because `status_start` <= `response_line` size.
            // If `status_start` == `response_line` size, then `status` will be an empty slice.
            let status = &response_line[status_start..];

            if let Some(code_end) = find(status, &[SP]) {
                // Slice access is safe because `find` validates that `code_end` < `status` size.
                let code = &status[..code_end];

                // `text_start` <= `status` size.
                let text_start = code_end.checked_add(1).ok_or(ResponseError::Overflow)?;

                // Slice access is safe because `text_start` <= `status` size.
                let text = &status[text_start..];

                return Ok((version, code, text));
            }
        }

        Err(ResponseError::InvalidResponse)
    }

    /// Tries to parse a byte stream in a response line. Fails if the response line is malformed.
    ///
    /// # Errors
    /// `InvalidStatusCode` is returned if the specified HTTP status code is unsupported.
    /// `InvalidHttpVersion` is returned if the specified HTTP version is unsupported.
    /// `InvalidUri` is returned if the specified Uri is not valid.
    pub fn try_from(response_line: &[u8]) -> Result<Self, ResponseError> {
        let (version, code, text) = Self::parse_status_line(response_line)?;

        Ok(Self {
            http_version: Version::try_from(version)?,
            status_code: StatusCode::try_from(code, text)?,
        })
    }

    // Returns the minimum length of a valid response. The response must contain
    // the HTTP version(HTTP/DIGIT.DIGIT), the status code (200), the status text (minimum 2 character),
    // 2 separators (SP).
    fn min_len() -> usize {
        // Addition is safe because these are small constants.
        Version::Http10.raw().len() + 3 + StatusCode::OK.raw().len() + 2
    }
}

/// Wrapper over an HTTP Response.
///
/// The Response is created using a `Version` and a `StatusCode`. When creating a Response object,
/// the body is initialized to `None` and the header is initialized with the `default` value. The body
/// can be updated with a call to `set_body`. The header can be updated with `set_content_type` and
/// `set_server`.
#[derive(Debug, PartialEq)]
pub struct Response {
    status_line: StatusLine,
    headers: ResponseHeaders,
    body: Option<Body>,
}

/// Read response
impl Response {
    /// Creates a new HTTP `Response` with an empty body.
    ///
    /// Although there are several cases where Content-Length field must not
    /// be sent, micro-http omits Content-Length field when the response
    /// status code is 1XX or 204. If needed, users can remove it by calling
    /// `set_content_length(None)`.
    ///
    /// https://datatracker.ietf.org/doc/html/rfc9110#name-content-length
    /// > A server MAY send a Content-Length header field in a response to a
    /// > HEAD request (Section 9.3.2); a server MUST NOT send Content-Length
    /// > in such a response unless its field value equals the decimal number
    /// > of octets that would have been sent in the content of a response if
    /// > the same request had used the GET method.
    /// >
    /// > A server MAY send a Content-Length header field in a 304 (Not
    /// > Modified) response to a conditional GET request (Section 15.4.5); a
    /// > server MUST NOT send Content-Length in such a response unless its
    /// > field value equals the decimal number of octets that would have been
    /// > sent in the content of a 200 (OK) response to the same request.
    /// >
    /// > A server MUST NOT send a Content-Length header field in any response
    /// > with a status code of 1xx (Informational) or 204 (No Content). A
    /// > server MUST NOT send a Content-Length header field in any 2xx
    /// > (Successful) response to a CONNECT request (Section 9.3.6).
    pub fn new(http_version: Version, status_code: StatusCode) -> Self {
        let mut headers = ResponseHeaders::default();
        headers.set_content_length(match status_code {
            StatusCode::Continue | StatusCode::NoContent => None,
            _ => Some(0),
        });

        Self {
            status_line: StatusLine::new(http_version, status_code),
            headers,
            body: Default::default(),
        }
    }

    /// Updates the body of the `Response`.
    ///
    /// This function has side effects because it also updates the headers:
    /// - `ContentLength`: this is set to the length of the specified body.
    pub fn set_body(&mut self, body: Body) {
        self.headers.set_content_length(Some(body.len() as i32));
        self.body = Some(body);
    }

    /// Updates the content length of the `Response`.
    ///
    /// It is recommended to use this method only when removing Content-Length
    /// field if the response status is not 1XX or 204.
    pub fn set_content_length(&mut self, content_length: Option<i32>) {
        self.headers.set_content_length(content_length);
    }

    /// Updates the content type of the `Response`.
    pub fn set_content_type(&mut self, content_type: MediaType) {
        self.headers.set_content_type(content_type);
    }

    /// Marks the `Response` as deprecated.
    pub fn set_deprecation(&mut self) {
        self.headers.set_deprecation();
    }

    /// Updates the encoding type of `Response`.
    pub fn set_encoding(&mut self) {
        self.headers.set_encoding();
    }

    /// Sets the HTTP response server.
    pub fn set_server(&mut self, server: &str) {
        self.headers.set_server(server);
    }

    /// Sets the HTTP allowed methods.
    pub fn set_allow(&mut self, methods: Vec<Method>) {
        self.headers.set_allow(methods);
    }

    /// Allows a specific HTTP method.
    pub fn allow_method(&mut self, method: Method) {
        self.headers.allow_method(method);
    }

    fn write_body<T: Write>(&self, mut buf: T) -> Result<(), WriteError> {
        if let Some(ref body) = self.body {
            buf.write_all(body.raw())?;
        }
        Ok(())
    }

    /// Writes the content of the `Response` to the specified `buf`.
    ///
    /// # Errors
    /// Returns an error when the buffer is not large enough.
    pub fn write_all<T: Write>(&self, mut buf: &mut T) -> Result<(), WriteError> {
        self.status_line.write_all(&mut buf)?;
        self.headers.write_all(&mut buf)?;
        self.write_body(&mut buf)?;

        Ok(())
    }
}

impl Response {
    /// Parses a byte slice into a HTTP Response.
    pub fn try_from(byte_stream: &[u8], max_len: Option<usize>) -> Result<Self, ResponseError> {
        // If a size limit is provided, verify the response length does not exceed it.
        if let Some(limit) = max_len {
            if byte_stream.len() >= limit {
                return Err(ResponseError::InvalidResponse);
            }
        }

        // The first line of the response is the Status Line. The line ending is CR LF.
        let status_line_end = match find(byte_stream, &[CR, LF]) {
            Some(len) => len,
            // If no CR LF is found in the stream, the response format is invalid.
            None => return Err(ResponseError::InvalidResponse),
        };

        // Slice access is safe because `find` validates that `status_line_end` < `byte_stream` size.
        let status_line_bytes = &byte_stream[..status_line_end];
        if status_line_bytes.len() < StatusLine::min_len() {
            return Err(ResponseError::InvalidResponse);
        }

        let status_line = StatusLine::try_from(status_line_bytes)?;

        // Find the next CR LF CR LF sequence in our buffer starting at the end on the Response
        // Line, including the trailing CR LF previously found.
        match find(&byte_stream[status_line_end..], &[CR, LF, CR, LF]) {
            // If we have found a CR LF CR LF at the end of the status Line, the response
            // is complete.
            Some(0) => Ok(Self {
                status_line,
                headers: ResponseHeaders::default(),
                body: None,
            }),
            Some(headers_end) => {
                // Parse the response headers.
                // Start by removing the leading CR LF from them.
                // The addition is safe because `find()` guarantees that `stastus_line_end`
                // precedes 2 `CRLF` sequences.
                let headers_start = status_line_end + CRLF_LEN;
                // Slice access is safe because starting from `status_line_end` there are at least two CRLF
                // (enforced by `find` at the start of this method).
                let headers_and_body = &byte_stream[headers_start..];
                // Because we advanced the start with CRLF_LEN, we now have to subtract CRLF_LEN
                // from the end in order to keep the same window.
                // Underflow is not possible here because `byte_stream[status_line_end..]` starts with CR LF,
                // so `headers_end` can be either zero (this case is treated separately in the first match arm)
                // or >= 3 (current case).
                let headers_end = headers_end - CRLF_LEN;
                let headers_end = headers_end - CRLF_LEN;
                // Slice access is safe because `headers_end` is checked above
                // (`find` gives a valid position, and  subtracting 2 can't underflow).
                let headers = ResponseHeaders::try_from(&headers_and_body[..headers_end])?;

                // Parse the body of the response.
                // Firstly check if we have a body.
                let body = match headers.content_length() {
                    0 => {
                        None
                    }
                    content_length => {
                        // Multiplication is safe because `CRLF_LEN` is a small constant.
                        // Addition is also safe because `headers_end` started out as the result
                        // of `find(<something>, CRLFCRLF)`, then `CRLF_LEN` was subtracted from it.
                        let crlf_end = headers_end + 2 * CRLF_LEN;
                        // This can't underflow because `headers_and_body.len()` >= `crlf_end`.
                        let body_len = headers_and_body.len() - crlf_end;
                        // Headers suggest we have a body, but the buffer is shorter than the specified
                        // content length.
                        if body_len < content_length as usize {
                            return Err(ResponseError::InvalidResponse);
                        }
                        // Slice access is safe because `crlf_end` is the index after two CRLF
                        // (it is <= `headers_and_body` size).
                        let body_as_bytes = &headers_and_body[crlf_end..];
                        // If the actual length of the body is different than the `Content-Length` value
                        // in the headers, then this response is invalid.
                        if body_as_bytes.len() == content_length as usize {
                            Some(Body::new(body_as_bytes))
                        } else {
                            return Err(ResponseError::InvalidResponse);
                        }
                    }
                };
                Ok(Self {
                    status_line,
                    headers,
                    body,
                })
            },
            // If we can't find a CR LF CR LF even though the response should have headers
            // the response format is invalid.
            None => Err(ResponseError::InvalidResponse),
        }
    }

    /// Returns the Status Code of the Response.
    pub fn status(&self) -> StatusCode {
        self.status_line.status_code
    }

    /// Returns the Body of the response. If the response does not have a body,
    /// it returns None.
    pub fn body(&self) -> Option<Body> {
        self.body.clone()
    }

    /// Returns the Content Length of the response.
    pub fn content_length(&self) -> i32 {
        self.headers.content_length()
    }

    /// Returns the Content Type of the response.
    pub fn content_type(&self) -> MediaType {
        self.headers.content_type()
    }

    /// Returns the deprecation status of the response.
    pub fn deprecation(&self) -> bool {
        self.headers.deprecation()
    }

    /// Returns the HTTP Version of the response.
    pub fn http_version(&self) -> Version {
        self.status_line.http_version
    }

    /// Returns the allowed HTTP methods.
    pub fn allow(&self) -> Vec<Method> {
        self.headers.allow()
    }
}
