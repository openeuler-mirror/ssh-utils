#[macro_export]
macro_rules! debug_log {
    ($file:expr, $($arg:tt)*) => {
        if cfg!(debug_assertions) {
            use tokio::io::AsyncWriteExt;
            let mut debug_file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open($file)
                .await
                .expect("无法打开日志文件");
            debug_file.write_all(format!($($arg)*).as_bytes()).await.expect("写入失败");
            debug_file.write_all(b"\n").await.expect("写入失败");
        }
    };
}