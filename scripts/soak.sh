#!/usr/bin/env bash
set -euo pipefail
[[ $# -ge 1 ]] || { echo "usage: soak.sh EXECUTABLE [DURATION_SECONDS] [SAMPLE_SECONDS] [OUTPUT]" >&2; exit 2; }
executable="$(realpath "$1")"
duration="${2:-7200}"
interval="${3:-60}"
output="${4:-verge-soak.csv}"
[[ -x "$executable" && "$duration" -gt 0 && "$interval" -gt 0 ]] || { echo "invalid executable or duration" >&2; exit 2; }
"$executable" &
pid=$!
trap 'kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true' EXIT
printf 'timestamp,rss_bytes,handles\n' > "$output"
started=$(date +%s)
deadline=$((started+duration))
samples=0
while (( $(date +%s) < deadline )); do
  sleep "$interval"
  kill -0 "$pid" 2>/dev/null || { wait "$pid"; echo "process exited during soak" >&2; exit 1; }
  rss_kb="$(ps -o rss= -p "$pid" | tr -d ' ')"
  if [[ -d "/proc/$pid/fd" ]]; then handles="$(find "/proc/$pid/fd" -mindepth 1 -maxdepth 1 | wc -l)"; else handles="$(lsof -p "$pid" 2>/dev/null | wc -l)"; fi
  printf '%s,%s,%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$((rss_kb*1024))" "$handles" >> "$output"
  samples=$((samples+1))
done
(( samples > 0 )) || { echo "no soak samples collected" >&2; exit 1; }
tail -n 5 "$output" | awk -F, 'NR==1{first=$2;prev=$2;next}{if($2<prev)mono=1;prev=$2}END{if(NR==5 && mono!=1 && prev-first>26214400)exit 1}' || { echo "resident memory grew monotonically by more than 25 MiB" >&2; exit 1; }
echo "PASS: $samples soak samples written to $output"
