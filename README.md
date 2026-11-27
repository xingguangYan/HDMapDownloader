# HD Map Downloader v2.0

## 简介 / Introduction

一个功能强大的高清地图下载工具，整合了全球20多个免费地图源，支持多格式导出。

An powerful HD map downloader supporting 20+ free global tile sources.

## 功能特性 / Features

- 多数据源支持：谷歌、天地图、高德、百度、Esri、OSM、NASA等
- 精确选择区域：矩形、多边形选区
- 并发下载：可配置线程数，支持断点续传
- 导出格式：GeoTIFF、MBTiles
- 坐标系统：支持 WGS84、Web Mercator、CGCS2000
- 中文界面 + 打赏模块

## 安装 / Installation

### 从源码构建 / Build from Source

```bash
cargo build --release
```

### 使用 Docker / Using Docker

```bash
docker build -t hdmapdl .
docker run -it --rm hdmapdl
```

## 使用方法 / Usage

```bash
./hdmapdl
```

启动后选择地图层，框选区域，调整下载级别，点击"开始下载"。

## 数据源 / Sources

完整的开源数据源配置位于 `data-sources.yaml`，包括：

- Google Maps (卫星/地形，可能受地理限制)
- Esri World Imagery (全球高清影像)
- OpenStreetMap (矢量)
- NASA Blue Marble (全球底图)
- USGS National Map (美国)
- 天地图 (需 Key，精度高)
- 高德/百度/腾讯 (国内)
- 以及更多免费源

## 编译依赖 / Build Dependencies

- Rust 1.56+
- 可选：CMake (用于部分 rust crate)
- 可选：Windows SDK / MSYS2 / MinGW

## 授权 / License

MIT License

## 打赏 / Donate

本工具完全免费，完全由个人开发。如果您觉得有帮助，欢迎打赏支持：

请使用软件内的"显示收款码"功能。

## 贡献 / Contributing

欢迎提交 Issue 和 Pull Request。

## 注意 / Notes

- 部分数据源需要联网，请遵守各数据源的使用条款
- 不可用于商业用途或违反相关法律法规
- 下载的数据仅供参考，使用请自行评估

--- ="-" 

# HD Map Downloader v2.0

[Setup instructions for Windows / macOS / Linux], [Troubleshooting], [Roadmap], [Changelog]
