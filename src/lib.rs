pub mod app;
pub mod config;
pub mod helper;
pub mod macros;
pub mod ssh;
pub mod widgets;

// 导出需要测试的模块和函数
pub use ssh::key_session::KeySession;
pub use ssh::ssh_session::{AuthMethod, SshSession};