#!/bin/sh
set -eu
[ "$#" = 2 ] && [ "$1" = --onion ] && [ -d "$2" ]
printf '%s\n' "$$" >"$CLOCK_TEST_ROOT/game-pid"
case "${CLOCK_TEST_GAME:-normal}" in
    normal) exit 0 ;;
    ondemand-scale)
        printf '1200\n' >"$CLOCK_TEST_ROOT/frequency"
        exit 0
        ;;
    failed) exit 7 ;;
    runtime-clock)
        : >"$BALATRO_CLOCK_MARKER"
        "$BALATRO_SYSTEM_DIR/bin/cpuclock" 1500 >/dev/null
        exit 0
        ;;
    runtime-clock-eco)
        : >"$BALATRO_CLOCK_MARKER"
        "$BALATRO_SYSTEM_DIR/bin/cpuclock" 1500 >/dev/null
        "$BALATRO_SYSTEM_DIR/bin/cpuclock" 1200 >/dev/null
        exit 0
        ;;
    signal)
        trap 'exit 0' TERM
        while :; do sleep 0.05; done
        ;;
esac
