# shellcheck shell=sh

isolate() {
	export HOME="$PWD"
	export XDG_CONFIG_HOME="$PWD/config"
	export XDG_CACHE_HOME="$PWD/cache"
	export TMPDIR="$PWD/tmp"
	export TZ=UTC
	export COLUMNS=100
	# Exported empty, which linea reads as unset: an unset does not outlive a scrut block.
	for name in LINEA_CONFIG CLAUDE_CONFIG_DIR CLAUDE_CODE_EFFORT_LEVEL \
		HERDR_ENV HERDR_PANE_ID HERDR_SOCKET_PATH; do
		export "$name="
	done
	mkdir -p "$XDG_CONFIG_HOME/linea" "$XDG_CACHE_HOME" "$TMPDIR" plain
	cat >"$XDG_CONFIG_HOME/linea/linea.toml" <<'TOML'
[appearance]
mode = "dark"
[sections.model]
icon = "M"
[sections.effort]
icon = "E"
[sections.project]
icon = "P"
[sections.branch]
icon = "B"
[sections.pane]
icon = "H"
[sections.duration]
icon = "D"
[sections.cache]
icon = "C"
[sections.misses]
icon = "X"
[sections.context]
icons = ["0", "1", "2", "3", "4", "5", "6", "7", "8"]
TOML
}

# A git repository named $1 on branch $2.
make_repo() {
	git init -q -b "$2" "$1"
	git -C "$1" -c user.email=test@example.com -c user.name=Test -c commit.gpgsign=false \
		commit -q --allow-empty -m init
}

# Claude Code's status JSON for directory $1, with jq assignments in $2.
session() {
	jq -cn --arg dir "$1" '{
		model: {display_name: "Opus", id: "claude-opus"},
		session_id: "s1",
		workspace: {current_dir: $dir},
		context_window: {total_input_tokens: 45200, context_window_size: 200000, used_percentage: 22}
	} | '"${2:-.}"
}

# The line linea draws, without colour.
line() {
	linea | sed 's/\x1b\[[0-9;]*m//g'
}
