#!/bin/sh
if [ "$1" = "--version" ]; then echo 'fake-cursor 1.0.0'; exit 0; fi
printf '%s\n' "$@" > "${CRAFTEL_RECORD:?}/argv"
pwd > "$CRAFTEL_RECORD/cwd"
printf '{"type":"system","session_id":"fake-session","request_id":"fake-request"}\n'
printf '{"type":"assistant","text":"frag'
sleep "${CRAFTEL_DELAY:-0}"
printf 'mented"}\n'
printf 'not json\n'
printf '{"type":"result","result":"done"}\n'
printf '%s' "${CRAFTEL_STDERR:-fake stderr}" >&2
sleep "${CRAFTEL_HOLD:-0}"
exit "${CRAFTEL_EXIT:-0}"
