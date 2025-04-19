# ppt_stealer-rs

针对国内授课场景，将本地桌面已有的、新增的 PPT, DOC, PDF 自动上传到远程 SSH 服务器。

## 项目介绍

课讲得烂固然糟糕，不公开课件更是雪上加霜。这个程序奈何不了某些渣滓，但可以帮你拿到有用的学习资料。

在课前提前运行这个程序，它可以将已有的、新增的文档自动上传到你指定的远程电脑/服务器。

## 项目优势

- 使用原生 Rust 编写，性能强劲，内存安全
- cargo 一键编译，方便快捷
- 支持 Windows, macOS, Linux 等多个操作系统

## 已知问题

- [ ] 若长时间不上传新文件，可能导致连接中断（取决于各linux发行版的策略），因而在上传新文件时在 `.unwrap()` 处 panic。预计在 v0.3 项目代码重构后解决这一问题。目前在需要时提前重启程序，防止程序崩溃。

## 参数

```plaintext
Usage: ppt_stealer-rs.exe [OPTIONS]

Options:
  -i, --ip <IP>
          SSH IP address or domain
  -p, --port <PORT>
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
      --remote-folder-name <REMOTE_FOLDER_NAME>
          Scan additional folder for files.
      --usb
          Scan USB for files.
  -L, --debug-level <DEBUG_LEVEL>
          Debug level. [default: info] [possible values: trace, debug, info, warn, error]
      --desktop-path <DESKTOP_PATH>
          Custimised desktop path
  -m, --min-depth <MIN_DEPTH>
          Minimum depth of file (included)
  -M, --max-depth <MAX_DEPTH>
          Maximum depth of file (included)
  -a, --add-paths <ADD_PATHS>
          Additional paths to scan
  -r, --regex <REGEX>
          Regex pattern to match files
      --formats <FORMATS>
          Assign file formats [default: "ppt pptx odp doc docx odt xls xlsx ods csv txt md"]
  -h, --help
          Print help
  -V, --version
          Print version
```

## 关于 SSH 服务器……

> SSH 是一种网络协议，用于计算机之间的加密登录。最早的时候，互联网通信都是明文通信，一旦被截获，内容就暴露无疑。1995 年，芬兰学者 Tatu Ylonen 设计了 SSH 协议，将登录信息全部加密，成为互联网安全的一个基本解决方案，迅速在全世界获得推广，目前已经成为 Linux 系统的标准配置。

简单来说，SSH 就是用于和其他电脑通过网络进行连接的工具。

除了系统和相关工具，SSH 还需要远程（被连接的）服务器具有公网 IP。鉴于国内网络现状，民用 IPv4 已经告罄，尽管公网 IPv6 在家庭宽带中已经很常见了，但很多学校（的部分教室）并不支持 IPv6，获得公网 IP 的最稳妥的方式是租各大云服务厂商（阿里，腾讯，华为）的云服务器。具体配置需求可以咨询工作人员。

## 编译

注：各个分支（包括 main）的版本并不稳定，请使用发行版或标签的源代码进行编译。

1. 安装 rustc 和 cargo
   前往 [rust-lang.org/install](https://www.rust-lang.org/tools/install) 下载、安装 Rust 工具链。
2. cd <项目根目录>
3. cargo build --release
4. 二进制文件位于 target/release/ 中

## 依赖

rustc 1.83.0
cargo 1.83.0

```toml
[dependencies]
dirs = "=5.0.1"
clap = { version = "=4.5.23", features = ["derive"] }
log = "=0.4.22"
env_logger = "=0.9"
walkdir = "=2.5.0"
sha2 = "=0.10.8"
ssh2 = "=0.9.4"
ctrlc = "=3.4.5"
chrono = "=0.4.39"
sysinfo = "=0.33.0"
regex = "=1.11.1"
```

## 计划

- [x] 去除缓冲文件 (will be supported in v0.2 final release)
- [x] 识别 U 盘，并上传其中所有的文档文件 (will be supported in v0.2 final release)
- [x] 解决上传时 U 盘弹出导致路径不存在、程序 panic 的问题 (will be supported in v0.2 final release)
- [ ] 添加将文件复制到本地特定目录的功能 (will be supported in v0.3.2)
- [x] 添加额外的本地目录 (will be supported in v0.3)
- [ ] 添加基于 Slint 客户端
- [ ] 添加隐藏命令行窗口的模式
- [ ] 添加对 ftp 服务器的支持
- [x] 在云端保留原文件相对桌面的相对路径 (will be supported in v0.2 final release)
- [x] 检测到远程同名文件内容相同后，取消重复上传
- [x] 指定路径，代替默认的桌面路径 (will be supported in v0.3)
- [x] 指定在目录搜索文件的最小、最大目录深度 (will be supported in v0.3)
- [x] 指定额外的文件格式 (will be supported in v0.3)
- [x] 通过正则表达式指定文件 (will be supported in v0.4)
