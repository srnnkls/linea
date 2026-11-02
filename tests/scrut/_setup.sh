# shellcheck shell=sh

isolate() {
	export HOME="$PWD" XDG_CONFIG_HOME="$PWD/config" XDG_CACHE_HOME="$PWD/cache" \
		TMPDIR="$PWD/tmp" TZ=UTC COLUMNS=100
	# Exported empty, which linea reads as unset: an unset does not outlive a scrut block.
	for name in LINEA_CONFIG CLAUDE_CONFIG_DIR CLAUDE_CODE_EFFORT_LEVEL \
		HERDR_ENV HERDR_PANE_ID HERDR_SOCKET_PATH; do
		export "$name="
	done
	mkdir -p "$TMPDIR"
}
