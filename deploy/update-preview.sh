#!/usr/bin/env bash
# 下载 server 工作流产物，替换预览服务并重启。
# 用法：update-preview.sh [RUN_ID]
#   不传 RUN_ID 时取最新一次成功的构建；传入时部署指定构建（推荐在 gh run watch 之后传入，
#   避免 GitHub API 状态延迟导致拿到上一次的产物）。
set -euo pipefail

REPO=Camvanaa/shiyin
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

RUN=${1:-}
if [ -z "$RUN" ]; then
  RUN=$(gh run list -R "$REPO" --workflow server --status success --limit 1 --json databaseId -q '.[0].databaseId')
  [ -n "$RUN" ] || { echo "没有成功的 server 构建" >&2; exit 1; }
fi

gh run download "$RUN" -R "$REPO" -n shiyin-server-linux-x64 -D "$TMP"
sudo install -m 0755 "$TMP/shiyin" /opt/shiyin/shiyin
sudo systemctl restart shiyin
sleep 1
systemctl is-active shiyin
echo "已部署构建 $RUN"
