#!/bin/sh
set -eu

if [ "${1:-}" = "--version" ]; then
  echo 'craftel-automation-fixture 1.0'
  exit 0
fi

# CursorHarness supplies the generated prompt as its final argument. These
# labels are part of the prompt contract and make parsing deliberately narrow.
for argument in "$@"; do prompt=$argument; done
normalized=$(printf '%b' "$prompt")
task=$(printf '%s\n' "$normalized" | sed -n 's/^Task ID: //p')
project=$(printf '%s\n' "$normalized" | sed -n 's/^Project ID: //p')
stage=$(printf '%s\n' "$normalized" | sed -n 's/^Current stage: //p')
test -n "$task" && test -n "$project" && test -n "$stage"

printf '{"type":"system","session_id":"fixture-%s","request_id":"request-%s","model":"test"}\n' "$stage" "$stage"
printf '{"type":"assistant","text":"received generated prompt"}\n'

case "${CRAFTEL_AUTOMATION_MODE:-pass}" in
  pass|pass_nonzero)
    "$CRAFTEL_TEST_BIN" pass "$task" --project "$project" >/dev/null
    ;;
  fail)
    "$CRAFTEL_TEST_BIN" fail "$task" --project "$project" >/dev/null
    ;;
  none) ;;
  hold)
    sleep 10
    ;;
  delayed_none)
    sleep 0.25
    ;;
  *) echo "unknown CRAFTEL_AUTOMATION_MODE" >&2; exit 64 ;;
esac

printf '{"type":"result","result":"fixture complete"}\n'
test "${CRAFTEL_AUTOMATION_MODE:-pass}" != pass_nonzero || exit 17
