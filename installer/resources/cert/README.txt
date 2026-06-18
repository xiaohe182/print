HTTPS 证书目录
================

本目录用于存放 HePrint 的 HTTPS 证书。

首次启动时（v1 P4 阶段实现）：
- heprint-ca.pem       自签 CA 根证书（安装到系统受信任根）
- heprint-server.pem   服务端证书
- heprint-server.key   服务端私钥

当前 v1.0.0 暂未启用 HTTPS，仅支持 HTTP（端口 18000）。
