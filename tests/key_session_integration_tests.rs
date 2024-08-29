#[cfg(feature = "integration_tests")]
mod tests {
    use russh_keys::load_secret_key;
use ssh_utils_lib::{
    ssh::key_session::KeySession,
    ssh::ssh_session::{AuthMethod, SshSession},
};
use std::env;

#[tokio::test]
async fn test_key_session_integration() {
    let user = env::var("SSH_TEST_USER").expect("SSH_TEST_USER not set");
    let key_path = env::var("SSH_TEST_KEY_PATH").expect("SSH_TEST_KEY_PATH not set");
    let addr = env::var("SSH_TEST_ADDR").expect("SSH_TEST_ADDR not set");

    let key = load_secret_key(key_path.clone(), None).expect("Failed to load secret key");
    let auth = AuthMethod::Key(key);
    
    let mut session = KeySession::connect(user, auth, addr).await.expect("Failed to connect");

    // 测试执行命令
    let exit_code = session.call("echo 'Hello, World!'").await.expect("Failed to execute command");
    assert_eq!(exit_code, 0);

    // 关闭会话
    session.close().await.expect("Failed to close session");
}
}