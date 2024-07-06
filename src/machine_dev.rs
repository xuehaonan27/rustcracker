use http::{uri::Authority, HeaderMap, Method, Request, StatusCode, Uri};
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;

/// 给每一个firecracker实例一个单独的path? 这个在Provider那里做

/// Firecracker agent
pub struct Agent {
    socket_path: String,
    stream: UnixStream,
}

#[derive(Serialize, Deserialize)]
pub struct FirecrackerRequest {
    #[serde(with = "http_serde::method")]
    method: Method,

    #[serde(with = "http_serde::status_code")]
    status: StatusCode,

    #[serde(with = "http_serde::uri")]
    uri: Uri,

    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,

    #[serde(with = "http_serde::authority")]
    authority: Authority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerResponse {
    
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("firecracker connection failed: {0}")]
    Connection(String),
    #[error("firecracker communication failed: {0}")]
    Communication(String),
}

pub type AgentResult<T> = std::result::Result<T, AgentError>;

impl Agent {
    async fn communicate_with_firecracker(&self, request: FirecrackerRequest) -> AgentResult<()> {

        
        todo!()
    }
}



async fn communicate_with_firecracker(
    socket_path: &str,
    request: FirecrackerRequest,
) -> Result<FirecrackerResponse, Box<dyn Error>> {
    let mut stream = UnixStream::connect(socket_path).await?;

    // 序列化请求为 JSON
    let request_json = serde_json::to_string(&request)?;
    stream.write_all(request_json.as_bytes()).await?;

    // 读取响应
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;

    // 反序列化响应
    let response: FirecrackerResponse = serde_json::from_slice(&buf)?;

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let request = FirecrackerRequest {
        action: "DescribeInstance".to_string(),
    };

    let response = communicate_with_firecracker("/tmp/firecracker.socket", request).await?;
    println!("Response: {:?}", response);

    Ok(())
}
