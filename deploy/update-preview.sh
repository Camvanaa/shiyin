#!/usr/bin/env bash
# 下载最新一次成功的 server 工作流产物，替换预览服务并重启。
set -euo pipefail

REPO=Camvanaa/shiyin
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

RUN=$(gh run list -R "$REPO" --workflow server --status success --limit 1 --json databaseId -q '.[0].databaseId')
[ -n "$RUN" ] || { echo "没有成功的 server 构建" >&2; exit 1; }

gh run download "$RUN" -R "$REPO" -n shiyin-server-linux-x64 -D "$TMP"
sudo install -m 0755 "$TMP/shiyin" /opt/shiyin/shiyin
sudo systemctl restart shiyin
sleep 1
systemctl is-active shiyin
echo "已部署构建 $RUN"
