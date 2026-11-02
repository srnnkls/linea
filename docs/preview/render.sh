#!/bin/sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
font=${LINEA_PREVIEW_FONT:-$HOME/Library/Fonts/CascadiaCodeNF-Regular.ttf}
family=${LINEA_PREVIEW_FAMILY:-Cascadia Code NF}
linea="$root/target/release/linea"
scratch="$here/.cache"
mkdir -p "$scratch"
if [ ! -d "$scratch/linea/.git" ]; then
  git init -q -b main "$scratch/linea"
  git -C "$scratch/linea" -c user.name=linea -c user.email=linea@localhost -c commit.gpgsign=false commit -q --allow-empty -m preview
fi

line() {
  jq -c --arg dir "$scratch/linea" '.workspace.current_dir = $dir' "$here/session.json" |
    env LINEA_CONFIG="$here/$1.toml" COLUMNS="$2" TZ=UTC \
      HERDR_ENV=1 HERDR_PANE_ID=wK0:p1 HERDR_SOCKET_PATH=/ XDG_CACHE_HOME="$scratch" \
      "$linea"
}

shoot() {
  line "$2" "$3" > "$scratch/$1.ansi"
  freeze --execute "cat $scratch/$1.ansi" --output "$here/$1.png" \
    --font.file "$font" --font.family "$family" --font.size 14 --background "$4" --width "$5" \
    --padding 14,20 --margin 0 --border.radius 6 --window=false </dev/null
}

shoot dark dark 124 '#282c34' 1060
shoot light light 124 '#fafafa' 1060
shoot narrow dark 64 '#282c34' 580
