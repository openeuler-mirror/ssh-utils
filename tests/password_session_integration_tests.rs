#[cfg(feature = "integration_tests")]
mod tests {
    use ssh_utils_lib::{
        ssh::password_session::PasswordSession,
        ssh::ssh_session::{AuthMethod, SshSession},
    };
    use std::env;

    #[tokio::test]
    async fn test_password_session_integration() {
        let user = env::var("SSH_TEST_USER").expect("SSH_TEST_USER not set");
        let password = env::var("SSH_TEST_PASSWORD").expect("SSH_TEST_PASSWORD not set");
        let addr = env::var("SSH_TEST_ADDR").expect("SSH_TEST_ADDR not set");

        let auth = AuthMethod::Password(password);
        
        let mut session = PasswordSession::connect(user, auth, addr).await.expect("Failed to connect");

        // 测试执行命令
        let exit_code = session.call("echo 'Hello, World!'").await.expect("Failed to execute command");
        assert_eq!(exit_code, 0);

        // 关闭会话
        session.close().await.expect("Failed to close session");
    }
}