# scripts/traz-shell-hook.zsh
#
# Idempotent Zsh hook for traz shell failure tracking.
# Automatically captures failed build/test commands and logs them to traz.

if [ -z "$_TRAZ_SHELL_HOOK_ZSH_LOADED" ]; then
_TRAZ_SHELL_HOOK_ZSH_LOADED=1

_TRAZ_ENDPOINT="${TRAZ_ENDPOINT:-http://localhost:4000}"
_TRAZ_LAST_CMD=""

_traz_escape_json() {
  printf '%s' "$1" | awk '
    BEGIN { ORS = "" }
    {
      gsub(/\\/, "\\\\")
      gsub(/"/, "\\\"")
      gsub(/\t/, "\\t")
      gsub(/\r/, "\\r")
      if (NR > 1) { print "\\n" }
      print
    }
  '
}

_traz_preexec() {
  _TRAZ_LAST_CMD="$1"
}

_traz_precmd() {
  local exit_code=$?
  
  if [ -z "$_TRAZ_LAST_CMD" ]; then
    return
  fi

  if [ "$exit_code" -eq 0 ]; then
    # Clear last command to avoid double triggering on empty return key
    _TRAZ_LAST_CMD=""
    return
  fi

  # Trim leading whitespace
  local cmd="$_TRAZ_LAST_CMD"
  cmd="${cmd#"${cmd%%[![:space:]]*}"}"

  local matched=0
  case "$cmd" in
    cargo*|npm*|yarn*|pnpm*|pytest*|"python -m pytest"*|"go build"*|"go test"*|make*|cmake*|gradle*|mvn*)
      matched=1
      ;;
  esac

  if [ "$matched" -eq 0 ]; then
    _TRAZ_LAST_CMD=""
    return
  fi

  # Extract tool name (first word)
  local tool_name="${cmd%% *}"

  # Verify curl is installed
  if ! command -v curl >/dev/null 2>&1; then
    _TRAZ_LAST_CMD=""
    return
  fi

  # Format JSON payload
  local esc_title
  esc_title=$(_traz_escape_json "$_TRAZ_LAST_CMD failed (exit $exit_code)")
  
  local esc_summary
  esc_summary=$(_traz_escape_json "Command '$_TRAZ_LAST_CMD' failed with exit status $exit_code. Output capture is not enabled to avoid terminal interference.")

  local esc_cwd
  esc_cwd=$(_traz_escape_json "$PWD")

  local payload
  payload="{\"tool\":\"$tool_name\",\"event_type\":\"build_failure\",\"title\":\"$esc_title\",\"summary\":\"$esc_summary\",\"tags\":\"shell,failure,$tool_name\",\"metadata\":{\"exit_code\":$exit_code,\"cwd\":\"$esc_cwd\",\"shell\":\"zsh\"}}"

  # Send to traz asynchronously, never block, silent
  curl --max-time 2 --silent --fail \
    -H "Content-Type: application/json" \
    -d "$payload" \
    "${TRAZ_ENDPOINT:-$_TRAZ_ENDPOINT}/events" >/dev/null 2>&1 || true

  # Clear last command to prevent repeating
  _TRAZ_LAST_CMD=""
}

# Register hooks idempotently into arrays
if [[ -z "${preexec_functions[(r)_traz_preexec]}" ]]; then
  preexec_functions+=(_traz_preexec)
fi

if [[ -z "${precmd_functions[(r)_traz_precmd]}" ]]; then
  precmd_functions+=(_traz_precmd)
fi

fi
