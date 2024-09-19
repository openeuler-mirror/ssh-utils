# ssh-utils
简体中文 | [English](./README.en.md)
<div align="center">

**ssh-utils 是一个帮助您快速建立 ssh 连接的命令行工具。**

![demo](https://gitee.com/YukinoCoco/ssh-utils/raw/assets/assets/demo.gif)

</div>

#### 介绍
在连接远程服务器过程中，每次输入命令连接时，都会花费一些时间。随着机器和用户的增多，每个机器和用户密码的不同，时间成本耗费越来越多。  
本工具提供一套命令行工具，执行后命令后可以通过上下箭头，选择想要连接的服务器和用户，大大缩短每次输入命令连接远程服务器的时间，方便又快捷。

#### 安装教程

##### 1. 通过 cargo 安装
```bash
# OpenEuler : dnf install openssl-devel
# Debian/Ubuntu : apt install libssl-dev
cargo install ssh-link
```

##### 2. 通过 Release 安装
从 release 页面下载对应平台的可执行文件或安装包安装。

##### 3. 通过源码安装
```bash
# OpenEuler : dnf install openssl-devel
# Debian/Ubuntu : apt install libssl-dev
git clone https://gitee.com/openeuler/ssh-utils
cd ssh-utils
cargo build --release
sudo cp target/release/ssh-utils /usr/bin/
```

#### 使用说明

安装之后，使用 ssh-utils 命令运行工具。添加服务器信息后，您可以通过小键盘上下箭头选择想要连接的服务器，回车后开始连接。  
你可以 `ssh-copy-id` 将公钥拷贝到远程服务器，添加服务器信息时，密码留空则使用本地的密钥连接，密钥的使用顺序同 OpenSSH `ssh` 命令的默认顺序一致。

#### 参与贡献

1.  Fork 本仓库
2.  新建 Feat_xxx 分支
3.  提交代码
4.  新建 Pull Request