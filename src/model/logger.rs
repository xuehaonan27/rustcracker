use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::Json;
/// logger can only be constructed once
/// and cannot update after configuration

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}
impl From<LogLevel> for String {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Debug => "Debug".into(),
            LogLevel::Error => "Error".into(),
            LogLevel::Info => "Info".into(),
            LogLevel::Warning => "Warning".into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Logger {
    // Set the level. The possible values are case-insensitive.
    // Enum: [Error Warning Info Debug]
    level: LogLevel,

    // Path to the named pipe or file for the human readable log output.
    // Required: true
    log_path: PathBuf,

    // Whether or not to output the level in the logs.
    show_level: Option<bool>,

    // Whether or not to include the file path and line number of the log's origin.
    show_log_origin: Option<bool>,
}

impl<'a> Json<'a> for Logger {
    type Item = Logger;
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            level: LogLevel::Warning,
            log_path: "".into(),
            show_level: None,
            show_log_origin: None,
        }
    }
}

impl Logger {
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn with_log_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.log_path = path.into();
        self
    }

    pub fn set_show_level(mut self, b: bool) -> Self {
        self.show_level = Some(b);
        self
    }
    
    pub fn set_show_origin(mut self, b: bool) -> Self {
        self.show_log_origin = Some(b);
        self
    }

    // 在指定path位置创建fifo文件并且封装配置返回
    // fn create_fifo(
    //     log_path: String,
    //     level: LogLevel,
    //     show_level: Option<bool>,
    //     show_log_origin: Option<bool>,
    // ) -> io::Result<Self> {
    //     let pipe = File::create(&log_path)?;
    //     Ok(Self {
    //         log_path,
    //         level,
    //         show_level,
    //         show_log_origin,
    //     })
    // }

    // /// 将Config指定的文件包含的内容输出到String里面
    // /// 对于fifo和file两种格式有两种处理方法
    // fn read_to_string(&self) -> Result<String, ConfigError> {
    //     self.validate()?;
    //     if let Some(path) = &self.file_path {
    //         let mut file_read = File::open(path)?;
    //         let mut buf = String::new();
    //         file_read.read_to_string(&mut buf)?;
    //         Ok(buf)
    //     } else if let Some(path) = &self.pipe_path {
    //         let mut file_read = File::open(path)?;
    //         let mut buf = String::new();
    //         file_read.read_to_string(&mut buf)?;
    //         Ok(buf)
    //     } else {
    //         Err(ConfigError { message: "Unknown".to_string() })
    //     }
    // }
}
