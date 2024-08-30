# ssh-utils
English | [简体中文](./README.md)
<div align="center">

**ssh-utils is a command-line tool that helps you quickly establish SSH connections.**

![demo](https://gitee.com/YukinoCoco/ssh-utils/raw/assets/assets/demo.gif)

</div>

#### Introduction
When connecting to remote servers, entering commands for each connection can be time-consuming. As the number of machines and users increases, along with different passwords for each, the time cost becomes increasingly significant.  

This tool provides a set of command-line utilities that allow you to select the server and user you want to connect to using arrow keys after executing the command, greatly reducing the time spent on inputting commands to connect to remote servers, making it convenient and efficient.

#### Installation Guide

##### 1. Install via cargo
```bash
cargo install ssh-utils
```

##### 2. Install from Release
Download the executable file or installation package for your platform from the release page.

##### 3. Install from source code
```bash
git clone https://gitee.com/openeuler/ssh-utils
cd ssh-utils
cargo build --release
sudo cp target/release/ssh-utils /usr/bin/
```

#### Usage Instructions

After installation, run the tool using the ssh-utils command. Once you've added server information, you can use the up and down arrow keys to select the server you want to connect to, and press Enter to start the connection.  

You can use `ssh-copy-id` to copy your public key to the remote server. When adding server information, leave the password blank to use local key-based authentication. The key usage order is consistent with the default order of the OpenSSH `ssh` command.

#### How to Contribute

1. Fork this repository
2. Create a new branch: Feat_xxx
3. Commit your code
4. Create a new Pull Request