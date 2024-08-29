.PHONY: test integration_tests

test:
	@echo "Running regular tests..."
	cargo test

integration_test:
	@echo "Please enter the following information for integration tests:"
	@read -p "SSH username: " ssh_user && \
	read -p "SSH key path: " ssh_key_path && \
	read -p "SSH server address: " ssh_addr && \
	stty -echo && \
	read -p "SSH password: " ssh_password && \
	stty echo && \
	echo && \
	export SSH_TEST_USER="$$ssh_user" && \
	export SSH_TEST_KEY_PATH="$$ssh_key_path" && \
	export SSH_TEST_ADDR="$$ssh_addr" && \
	export SSH_TEST_PASSWORD="$$ssh_password" && \
	echo "Running integration tests..." && \
	cargo test --features integration_tests -- --nocapture

build:
	cargo build --release