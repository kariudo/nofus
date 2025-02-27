# 🚀 Nofus - The NFS Mount Guardian

[![Rust](https://img.shields.io/badge/Rust-1.60%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI/CD](https://github.com/kariudo/nofus/actions/workflows/rust.yml/badge.svg)](https://github.com/kariudo/nofus/actions)

**Nofus** is a _🔥 blazingly-fast, 🧠 memory-safe, 🔋 batteries-included, 💺ergonomic, 🦀 100% Rust-powered_ daemon that vigilantly monitors your NFS mounts and triggers custom actions based on their availability. Never get caught with stale mounts again! 🛡️

<p align="center">
  <img src="https://media.giphy.com/media/3o7abKhOpu0NwenH3O/giphy.gif" alt="Important GIF" width="300"/>
</p>

## ✨ Features

- 🕵️ **Real-time NFS Mount Monitoring** using Linux `inotify`
- ⚡ **Configurable System Commands** for mount/unmount events
- 🧪 **Dry-Run Mode** for safe testing
- 📊 **Verbose Logging** for deep insights
- 🔄 **Periodic Health Checks** (configurable interval)
- 📁 **YAML Configuration** for easy setup

## 📦 Installation

1. **Prerequisites**: _If you want to run the project from source, or install from cargo directly._ Ensure you have Rust installed (1.60+)

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install Nofus**:

   ```bash
   cargo install nofus
   ```

## ⚙️ Configuration

Create `config.yml` in your `$HOME/.config/nofus` directory:

```yaml
# Sample Configuration
mount_points:
  - "/mnt/nfs/share1"
  - "/media/cloud_storage"

delay_seconds: 5  # Check interval

# Commands to execute (supports full shell syntax)
all_mounted_cmd: "systemctl start my-app.service"
any_unmounted_cmd: "systemctl stop my-app.service && wall 'NFS Crisis!'"
```

> Note: If you start nofus without creating a configuration file first,
> one will be created from a template and nofus will exit.

## 🚦 Usage

```bash
nofus [OPTIONS]
```

**Options**:

- `--dry-run`: Simulate without executing commands
- `--verbose`: Show debug-level logging

**Example**:

```bash
nofus --verbose --dry-run
```

## 🖥️ Sample Workflow

```text
2023-09-15T14:30:00 [INFO] Initial state: All NFS mounts available ✅
2023-09-15T14:35:22 [ERROR] NFS mount disconnected: /mnt/nfs/share1 ❌
2023-09-15T14:35:22 [DEBUG] Executing: systemctl stop my-app.service
2023-09-15T14:36:45 [INFO] Mount recovered: /mnt/nfs/share1 ✅
```

## 🤝 Contributing

We welcome contributions! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

<p align="center">
  Made with ❤️ by <a href="https://github.com/kariudo">kariudo</a> |
  ☕ <a href="https://buymeacoffee.com/kariudo">Support the developer</a>
</p>
