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
                .expect("can't open log file");
            debug_file.write_all(format!($($arg)*).as_bytes()).await.expect("failed to write log");
            debug_file.write_all(b"\n").await.expect("failed to write new line");
        }
    };
}