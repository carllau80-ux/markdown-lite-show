//! Document export backends.

pub mod docx;
pub mod pdf;

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("文件读写失败：{0}")]
    Io(#[from] io::Error),

    #[error(
        "未找到可用的浏览器内核。\n\n\
         PDF 导出需要 Microsoft Edge 或 Google Chrome/Chromium。\n\
         Windows 10/11 通常已自带 Edge；安装后无需额外配置，重新导出即可。"
    )]
    BrowserNotFound,

    #[error("PDF 生成失败：{0}")]
    Pdf(String),

    #[error("Word 文档生成失败：{0}")]
    Docx(String),
}
