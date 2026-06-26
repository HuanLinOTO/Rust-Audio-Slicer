# 🎵 Audio Slicer - 高性能音频切片工具

一个基于 Rust 开发的高性能音频切片工具，支持多线程并行处理、智能静音检测和批量文件处理。

> [!WARNING]  
> 该项目由 MetaSo + Claude Sonnet 4 生成，不保证可靠性

## ✨ 特性

- 🚀 **高性能处理**: RTF < 0.01，处理速度比实时播放快 130+倍
- 🧵 **多线程并行**: 支持多线程并行处理，可配置线程数
- 📊 **实时进度条**: 美观的进度条显示处理状态
- 🔍 **递归搜索**: 自动递归搜索目录中的所有音频文件
- 📁 **目录结构保留**: 完整保留原始目录层次结构
- 🎯 **智能静音检测**: 自动过滤静音片段，只保留有效音频
- 🎛️ **参数可配置**: 所有切片参数都可通过命令行调整
- 📈 **详细统计**: 提供 RTF、并行效率等详细性能指标

## 🎼 支持的音频格式

- WAV
- MP3
- FLAC
- M4A
- AAC
- OGG

## 🚀 快速开始

### 下载二进制文件

从 [Releases](https://github.com/your-repo/audio-slicer/releases) 页面下载适合您操作系统的预编译二进制文件：

- **Windows**: `audio-slicer-windows.exe`
- **Linux**: `audio-slicer-linux`
- **macOS**: `audio-slicer-macos`

### 从源码编译

如果您需要从源码编译，请确保已安装 [Rust](https://rustup.rs/)：

```bash
git clone https://github.com/HuanLinOTO/Rust-Audio-Slicer/
cd Rust-Audio-Slicer
cargo build --release
```

### 基本用法

```bash
# 使用下载的二进制文件处理单个文件
./audio-slicer slice -i input.wav -o slices

# 处理整个目录（递归搜索）
./audio-slicer slice -i audio_folder -o output_folder

# 使用4个线程并行处理
./audio-slicer slice -i audio_folder -o output_folder --threads 4

# 激进切片模式（极敏感的静音检测）
./audio-slicer slice -i input.wav -o output \
  --threshold-db -100 \
  --max-silence-ms 5
```

## 🔧 命令行参数

### 必需参数

- `-i, --input <PATH>`: 输入音频文件或目录路径
- `-o, --output <PATH>`: 输出目录路径

### 可选参数

- `-t, --threads <NUM>`: 并行处理线程数（默认为 CPU 核心数）
- `--threshold-db <DB>`: 静音阈值，单位 dB（默认: -55.0）
- `--min-length-ms <MS>`: 最小片段长度，单位毫秒（默认: 1000）
- `--min-interval-ms <MS>`: 最小间隔，单位毫秒（默认: 100）
- `--hop-size-ms <MS>`: 跳跃大小，单位毫秒（默认: 5）
- `--max-silence-ms <MS>`: 最大静音长度，单位毫秒（默认: 800）
- `--max-merge-duration-ms <MS>`: 最大合并时长，单位毫秒（默认: 8000）
- `--max-duration-ms <MS>`: 最大切片时长，单位毫秒。超过该时长的切片会被硬切成多块，每块严格小于该值；0 表示禁用（默认: 0）
- `--silence-threshold <FLOAT>`: 静音检测阈值（默认: 0.001）
- `--min-audio-ratio <FLOAT>`: 最小有效音频占比（默认: 0.1）

### 查看帮助

```bash
./audio-slicer slice --help
```

## 📋 使用示例

### 1. 基本切片处理

```bash
# 使用默认参数处理单个文件
./audio-slicer slice -i my_audio.wav -o output

# 处理目录并保留结构
./audio-slicer slice -i audio_dataset -o processed_audio
```

### 2. 自定义切片参数

```bash
# 更激进的切片（更多短片段）
./audio-slicer slice -i input.wav -o output \
  --threshold-db -60 \
  --min-length-ms 500 \
  --max-silence-ms 300

# 更保守的切片（更少长片段）
./audio-slicer slice -i input.wav -o output \
  --threshold-db -45 \
  --min-length-ms 3000 \
  --max-silence-ms 2000
```

### 3. 性能优化

```bash
# 小批量文件使用4线程
./audio-slicer slice -i small_dataset -o output --threads 4

# 大批量文件使用更多线程
./audio-slicer slice -i large_dataset -o output --threads 16
```

## 📁 输出结构

工具会完整保留原始目录结构，每个音频文件会生成一个对应的文件夹：

```
输入目录:
audio_dataset/
├── speaker1/
│   ├── recording1.wav
│   └── recording2.wav
└── speaker2/
    └── recording3.wav

输出目录:
processed_audio/
├── speaker1/
│   ├── recording1/
│   │   ├── slice_000.wav
│   │   ├── slice_001.wav
│   │   └── slice_002.wav
│   └── recording2/
│       ├── slice_000.wav
│       └── slice_001.wav
└── speaker2/
    └── recording3/
        ├── slice_000.wav
        ├── slice_001.wav
        └── slice_002.wav
```

## 📊 性能指标

### RTF (Real Time Factor)

- RTF < 0.01 表示处理速度比实时播放快 100 倍以上
- 典型性能：处理 3 分 48 秒音频仅需 1.4 秒

### 多线程效率

- 小批量文件（3-10 个）：推荐使用 4 线程，并行效率可达 70%+
- 大批量文件（100+个）：推荐使用 CPU 核心数的线程
- 最大加速比：约 3 倍（取决于文件数量和硬件配置）

### 示例输出

```
🚀 性能指标:
   - 整体RTF: 0.008x
   - 处理速度比实时播放快 132.5倍
   - 平均每个文件处理时间: 1.73s
   - 并行加速比: 2.94x (使用4线程)
   - 并行效率: 73.6%
```

## 🔧 高级配置

### 静音检测调优

- **threshold-db**: 越低越敏感，-60dB 会检测到非常微弱的静音
- **silence-threshold**: 线性阈值，0.001 约等于-60dB
- **min-audio-ratio**: 有效音频占比，0.1 表示至少 10%为有效音频

### 切片长度控制

- **min-length-ms**: 控制最短片段长度，避免过短的切片
- **max-merge-duration-ms**: 控制合并后的最大长度
- **max-duration-ms**: 控制单个切片的最大时长，超长切片会被硬切
- **hop-size-ms**: 分析精度，越小越精确但处理时间更长

## 🛠️ 开发说明

### 项目结构

```
src/
├── main.rs     # CLI界面和主程序逻辑
├── audio.rs    # 音频文件加载和格式转换
└── slicer.rs   # 切片算法和静音检测
```

### 核心技术

- **音频处理**: Symphonia 库，支持多种音频格式
- **并行处理**: Rayon 数据并行框架
- **进度显示**: Indicatif 进度条库
- **CLI**: Clap 命令行解析库

### 编译选项

```bash
# 开发模式（包含调试信息）
cargo build

# 发布模式（性能优化）
cargo build --release

# 运行测试
cargo test
```

## 📈 使用建议

### 参数调优指南

1. **语音数据处理**:

   ```bash
   --threshold-db -50 --min-length-ms 1000 --max-silence-ms 500
   ```

2. **音乐切片**:

   ```bash
   --threshold-db -60 --min-length-ms 2000 --max-silence-ms 1000
   ```

3. **播客/讲座**:
   ```bash
   --threshold-db -45 --min-length-ms 5000 --max-silence-ms 2000
   ```

### 性能优化建议

- 对于 SSD 存储，可以使用更多线程
- 对于机械硬盘，建议限制线程数为 4-8
- 内存不足时，减少并行线程数
- 处理大文件时，考虑增加 `hop-size-ms`

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 🙏 致谢

- [Symphonia](https://github.com/pdeljanov/Symphonia) - 音频解码库
- [Rayon](https://github.com/rayon-rs/rayon) - 数据并行库
- [Indicatif](https://github.com/console-rs/indicatif) - 进度条库
- [Clap](https://github.com/clap-rs/clap) - 命令行解析库
