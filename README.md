# ppt_stealer-rs

针对国内授课场景，将本地桌面已有的、新增的 PPT, DOC, PDF 自动上传到远程 SSH 服务器。

## 依赖

rustc 1.82.0
cargo 1.82.0

```toml
[dependencies]
dirs = "5.0.1"
clap = { version = "4.5.23", features = ["derive"] }
log = "0.4.22"
env_logger = "0.9"
walkdir = "2.5.0"
sha2 = "0.10.8"
ssh2 = "0.9.4"
ctrlc = "3.4.5"
chrono = "0.4.39"
```

## 参数

```plaintext
Usage: ppt_stealer-rs.exe [OPTIONS]

Options:
  -i, --ssh-ip <SSH_IP>
          SSH IP address or domain
  -p, --ssh-port <SSH_PORT>
          SSH IP port
  -u, --username <USERNAME>
          SSH username
  -P, --password <PASSWORD>
          SSH password
      --key-auth
          Use SSH key authentication. If not assigned, password authentication will be used.
      --refresh-interval <REFRESH_INTERVAL>
          Refresh interval in seconds [default: 30]
      --no-gui
          Assign no GUI mode
      --folder-name <FOLDER_NAME>
          Folder name for files
  -L, --debug-level <DEBUG_LEVEL>
          Debug level. Choose from trace, debug, info, warn and error [default: warn]
  -h, --help
          Print help
  -V, --version
          Print version

```

## 编译

1. 安装 rustc 和 cargo  
   前往 [rust-lang.org/install](https://www.rust-lang.org/tools/install) 下载、安装 Rust 工具链。
2. cd <项目根目录>
3. cargo build --release
4. 二进制文件位于 target/release/ 中
