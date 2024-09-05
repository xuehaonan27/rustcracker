mod buf;
mod buf_mut;
mod decode;
mod encode;
mod read_buf;

pub use buf::BufExt;
pub use buf_mut::BufMutExt;
pub use decode::ProtocolDecode;
pub use encode::ProtocolEncode;
pub use read_buf::ReadBuf;

#[cfg(not(feature = "_rt-tokio"))]
pub use futures_io::AsyncRead;

#[cfg(feature = "_rt-tokio")]
pub use tokio::io::AsyncRead;

#[cfg(not(feature = "_rt-tokio"))]
pub use futures_util::io::AsyncReadExt;

#[cfg(feature = "_rt-tokio")]
pub use tokio::io::AsyncReadExt;