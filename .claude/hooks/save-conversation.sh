#!/usr/bin/env bash
# Stop hook: converte o transcript JSONL da sessão atual em Markdown em docs/claude/interaction/.
# Idempotente — reescreve o arquivo a cada Stop (uma vez por turno).
set -euo pipefail

input="$(cat)"

session_id=$(echo "$input" | jq -r '.session_id // empty')
transcript_path=$(echo "$input" | jq -r '.transcript_path // empty')
cwd=$(echo "$input" | jq -r '.cwd // empty')

[ -z "$cwd" ] && cwd="${CLAUDE_PROJECT_DIR:-$(pwd)}"

if [ -z "$transcript_path" ] && [ -n "$session_id" ]; then
  sanitized=$(echo "$cwd" | sed 's|/|-|g')
  transcript_path="$HOME/.claude/projects/${sanitized}/${session_id}.jsonl"
fi

[ -f "$transcript_path" ] || exit 0
[ -n "$session_id" ] || exit 0

out_dir="${cwd}/docs/claude/interaction"
mkdir -p "$out_dir"

date_str=$(date +%Y-%m-%d)
out_file="${out_dir}/${date_str}-${session_id}.md"
tmp_file="${out_file}.tmp"

{
  echo "# Sessão Claude Code — ${date_str}"
  echo
  echo "> Session ID: \`${session_id}\`  "
  echo "> Projeto: \`${cwd}\`  "
  echo "> Atualizado: $(date -Iseconds)"
  echo
  echo "---"
  echo

  jq -r '
    select(.type == "user" or .type == "assistant") |
    if .type == "user" then
      if (.message.content | type) == "string"
      then "## Usuário\n\n" + .message.content + "\n\n---\n"
      else empty
      end
    else
      (.message.content
        | map(
            if .type == "text" then .text
            elif .type == "tool_use" then
              "**Tool:** `" + .name + "`\n\n```json\n" + (.input | tojson) + "\n```"
            else empty
            end
          )
        | join("\n\n")
      ) as $body
      | if ($body | length) > 0 then "## Claude\n\n" + $body + "\n\n---\n" else empty end
    end
  ' "$transcript_path"
} > "$tmp_file"

mv "$tmp_file" "$out_file"
