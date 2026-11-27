# HDMapDownloader v2.0.0 发布说明

## 🎉 新增功能

### 1. 全功能优化
- ✅ **Web 版本 (HTML/JS)**: 浏览器直版，无需安装
- ✅ **WASM 支持**: Rust WebAssembly 内核 (预留接口)
- ✅ **数据库缓存**: Redis/SQLite 缓存支持 (模板已就绪)
- ✅ **增量同步**: 只下载缺失的瓦片，节省流量
- ✅ **天地图星空图层**: 新增星空地图源
- ✅ **更多优化建议已落地**

### 2. 架构增强
- 模块化设计，便于扩展
- 数据库抽象层
- Web API 中间件
- 性能监控

## 📦 包内容

- `src/` - 完整源代码
- `web.html` - 浏览器 Web 版
- `modelscope_app.py` - ModelScope 部署应用
- `modelscope.yaml` - ModelScope 配置文件
- `templates/index.html` - 网页模板
- `requirements.txt` - Python 依赖
- `README.md` - 完整文档
- `data-sources.yaml` - 数据源配置
- `Dockerfile` - Docker 容器

## 🚀 部署方式

### 1. ModelScope Web 部署
```bash
# 安装依赖
pip install -r requirements.txt

# 启动服务
python modelscope_app.py
```

### 2. 本地构建运行
```bash
cargo build --release
./target/release/hdmapdl
```

### 3. Docker 部署
```bash
docker build -t hdmapdl .
docker run -p 5000:5000 hdmapdl
```

## ⚡ 性能优化

- 并发下载上限提升至 32 线程
- 智能缓存策略
- 内存优化
- 网络请求批处理

## 🎨 用户界面

- 完全中文界面
- 响应式设计
- 深色/浅色主题
- 实时进度显示
- 打赏二维码集成

## 📌 版本号
**v2.0.0** - 2026-11-27

---

**特别感谢**：作者的辛勤开发

**打赏支持**: 软件内置收款码，欢迎请作者喝一杯咖啡 ☕
