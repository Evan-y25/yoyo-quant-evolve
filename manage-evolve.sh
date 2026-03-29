#!/bin/bash

# yoyo 进化定时任务管理脚本
# 用法: ./manage-evolve.sh [status|start|stop|restart|logs|run]

PLIST="/Users/chao/Library/LaunchAgents/com.yoyo.evolve.plist"
LOG_FILE="/tmp/yoyo-evolve.log"
API_KEY="sk-0rOvQfoPkPPL2SV4V239S1zGIaoOysT7ZzQii4cgl5pP3oWs"
PROVIDER="apieasy"
PROJECT_DIR="/Users/chao/Project/yoyo-quant-evolve"

case "${1:-status}" in
  status)
    echo "【定时任务状态】"
    if launchctl list | grep -q "com.yoyo.evolve"; then
      echo "✅ 定时任务运行中"
      launchctl list | grep "com.yoyo.evolve"
    else
      echo "❌ 定时任务已停止"
    fi
    echo ""
    echo "【最后进化轮次】"
    cat "$PROJECT_DIR/ROUND_COUNT" 2>/dev/null || echo "未知"
    ;;

  start)
    echo "启动定时任务..."
    launchctl load "$PLIST"
    echo "✅ 定时任务已启动"
    ;;

  stop)
    echo "停止定时任务..."
    launchctl unload "$PLIST"
    echo "✅ 定时任务已停止"
    ;;

  restart)
    echo "重启定时任务..."
    launchctl unload "$PLIST" 2>/dev/null || true
    sleep 2
    launchctl load "$PLIST"
    echo "✅ 定时任务已重启"
    ;;

  logs)
    echo "【进化日志（实时）】"
    tail -f "$LOG_FILE"
    ;;

  run)
    echo "🚀 手动运行一次进化..."
    cd "$PROJECT_DIR"
    export API_KEY="$API_KEY"
    export PROVIDER="$PROVIDER"
    ./scripts/evolve.sh
    ;;

  *)
    echo "用法: $0 [status|start|stop|restart|logs|run]"
    echo ""
    echo "命令说明:"
    echo "  status   - 查看定时任务状态"
    echo "  start    - 启动定时任务"
    echo "  stop     - 停止定时任务"
    echo "  restart  - 重启定时任务"
    echo "  logs     - 查看实时日志"
    echo "  run      - 手动运行一次进化"
    ;;
esac
