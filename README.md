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
```

## 参数

```plaintext
Usage: ppt_stealer-rs [OPTIONS]

Options:
  -i, --ftp-ip <FTP_IP>
          FTP IP address or domain
  -p, --ftp-port <FTP_PORT>
          FTP IP port
  -u, --username <USERNAME>
          FTP username
  -P, --password <PASSWORD>
          FTP password
      --key-auth
          Use FTP key authentication. If not assigned, password authentication will be used.
      --refresh-interval <REFRESH_INTERVAL>
          Refresh interval in seconds [default: 30]
      --no-gui
          Assign no GUI mode
  -h, --help
          Print help
  -V, --version
          Print version

```
