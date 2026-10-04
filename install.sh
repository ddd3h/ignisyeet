#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
#
# IgnisYeet installer.
#
#   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --developer --cfd
#
# shellcheck disable=SC2016  # single-quoted scripts for `sh -c` are intentional
# Run with --help for all options.  Works with bash 3.2 (macOS) and later.
# Everything the installer runs is logged to <prefix>/share/ignisyeet/install.log.

set -Eeuo pipefail

# ----------------------------------------------------------------------------------------------
# Configuration (flags and IGNISYEET_* environment variables)
# ----------------------------------------------------------------------------------------------
REPO="${IGNISYEET_REPO:-ddd3h/ignisyeet}"
REPO_URL="${IGNISYEET_REPO_URL:-https://github.com/$REPO.git}"
DOWNLOAD_BASE="${IGNISYEET_DOWNLOAD_BASE:-https://github.com/$REPO/releases/download}"
CFD_ENV_NAME="ignisyeet-cfd"
CFD_PACKAGES="python=3.10 su2 gmsh python-gmsh openmpi"

ROLE="${IGNISYEET_ROLE:-}"                 # user | developer | "" (ask)
VERSION="${IGNISYEET_VERSION:-}"           # vX.Y.Z | "" (latest release)
PREFIX="${IGNISYEET_PREFIX:-$HOME/.local}"
DEV_DIR="${IGNISYEET_DIR:-}"
WANT_CFD="${IGNISYEET_CFD:-}"              # 1 | 0 | "" (ask; default no)
WANT_TEST=""                               # 1 | 0 | "" (ask; default yes)
ASSUME_YES="${IGNISYEET_YES:-0}"
NO_COLOR_FLAG="${IGNISYEET_NO_COLOR:-0}"
DRY_RUN="${IGNISYEET_DRY_RUN:-0}"
FORCE="${IGNISYEET_FORCE:-0}"
ADD_PATH="${IGNISYEET_ADD_PATH:-0}"
ACTION="install"                           # install | uninstall
PURGE=0

START_TIME=$(date +%s)
# Safe defaults until setup_ui runs (die() may be called while parsing arguments).
RESET=""; BOLD=""; RED=""; GREEN=""; YELLOW=""; BLUE=""; CYAN=""; ORANGE=""; GREY=""
G_ERR="x"; SHOW_CUR=""
LOG_READY=0
ERR_REPORTED=0

# ----------------------------------------------------------------------------------------------
# Terminal UI
# ----------------------------------------------------------------------------------------------
setup_ui() {
  USE_COLOR=0
  USE_UTF8=0
  IS_TTY=0
  [ -t 1 ] && IS_TTY=1
  if [ "$IS_TTY" = 1 ] && [ -z "${NO_COLOR:-}" ] && [ "$NO_COLOR_FLAG" != 1 ] && [ "${TERM:-dumb}" != dumb ]; then
    USE_COLOR=1
  fi
  case "${LC_ALL:-${LC_CTYPE:-${LANG:-}}}" in
    *[Uu][Tt][Ff]-8* | *[Uu][Tt][Ff]8*) USE_UTF8=1 ;;
  esac

  COLS=80
  if [ "$IS_TTY" = 1 ]; then
    COLS=$(tput cols 2>/dev/null || echo 80)
    [ "$COLS" -gt 100 ] && COLS=100
    [ "$COLS" -lt 50 ] && COLS=50
  fi
  BOXW=$((COLS - 4))
  [ "$BOXW" -gt 72 ] && BOXW=72

  if [ "$USE_COLOR" = 1 ]; then
    local e=$'\033'
    RESET="${e}[0m"; BOLD="${e}[1m"
    RED="${e}[38;5;203m"; GREEN="${e}[38;5;78m"; YELLOW="${e}[38;5;221m"
    BLUE="${e}[38;5;75m"; CYAN="${e}[38;5;80m"; ORANGE="${e}[38;5;209m"; GREY="${e}[38;5;245m"
    HIDE_CUR="${e}[?25l"; SHOW_CUR="${e}[?25h"; CLR="${e}[2K"
  else
    RESET=""; BOLD=""; RED=""; GREEN=""; YELLOW=""; BLUE=""; CYAN=""; ORANGE=""; GREY=""
    HIDE_CUR=""; SHOW_CUR=""; CLR=""
  fi

  if [ "$USE_UTF8" = 1 ]; then
    G_OK="✔"; G_ERR="✖"; G_WARN="⚠"; G_INFO="●"; G_SKIP="○"; G_ARROW="➜"; G_DOT="·"
    B_TL="╭"; B_TR="╮"; B_BL="╰"; B_BR="╯"; B_H="─"; B_V="│"
    BAR_ON="█"; BAR_OFF="░"
    SPIN_FRAMES="⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏"
  else
    G_OK="+"; G_ERR="x"; G_WARN="!"; G_INFO="*"; G_SKIP="-"; G_ARROW=">"; G_DOT="-"
    B_TL="+"; B_TR="+"; B_BL="+"; B_BR="+"; B_H="-"; B_V="|"
    BAR_ON="#"; BAR_OFF="."
    SPIN_FRAMES="| / - \\"
  fi
}

# Print a line to stdout (the terminal, even under `curl | bash`).
say() { printf '%s\n' "$*"; }
logf() { [ "$LOG_READY" = 1 ] && printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >>"$LOG_FILE" || true; }

ok()   { say "  ${GREEN}${G_OK}${RESET} $*"; logf "ok: $*"; }
warn() { say "  ${YELLOW}${G_WARN}${RESET} $*"; logf "warn: $*"; }
note() { say "  ${GREY}${G_DOT} $*${RESET}"; }
skip() { say "  ${GREY}${G_SKIP}${RESET} $*"; }
info() { say "  ${BLUE}${G_INFO}${RESET} $*"; logf "info: $*"; }

section() {
  say ""
  say "${BOLD}${ORANGE}${G_ARROW}${RESET} ${BOLD}$*${RESET}"
  logf "== $*"
}

# repeat STRING COUNT
repeat() { local i s=""; for ((i = 0; i < $2; i++)); do s="$s$1"; done; printf '%s' "$s"; }

strip_ansi() { printf '%s' "$1" | sed $'s/\033\\[[0-9;?]*[a-zA-Z]//g'; }

box_top() {
  local title="$1" plain
  plain=$(strip_ansi "$title")
  say "${GREY}${B_TL}${B_H} ${RESET}${BOLD}${title}${RESET}${GREY} $(repeat "$B_H" $((BOXW - ${#plain} - 5)))${B_TR}${RESET}"
}
box_row() {
  local plain pad
  plain=$(strip_ansi "$1")
  pad=$((BOXW - ${#plain} - 4))
  if [ "$pad" -lt 0 ]; then say "${GREY}${B_V}${RESET} $1"; return; fi # too long: leave the border open
  say "${GREY}${B_V}${RESET} $1$(repeat ' ' "$pad") ${GREY}${B_V}${RESET}"
}
# cmdpath PATH: like tilde, but with $HOME (safe inside double quotes when the user pastes the command)
cmdpath() { case "$1" in "$HOME"/*) printf '$HOME%s' "${1#"$HOME"}" ;; *) printf '%s' "$1" ;; esac; }
tilde() { case "$1" in "$HOME"/*) printf '~%s' "${1#"$HOME"}" ;; *) printf '%s' "$1" ;; esac; }
box_bottom() { say "${GREY}${B_BL}$(repeat "$B_H" $((BOXW - 2)))${B_BR}${RESET}"; }

banner() {
  local c1 c2 c3 c4 c5 on="█" sp=" "
  if [ "$USE_COLOR" = 1 ]; then
    c1=$'\033[38;5;196m'; c2=$'\033[38;5;202m'; c3=$'\033[38;5;208m'; c4=$'\033[38;5;214m'; c5=$'\033[38;5;220m'
  else
    c1=""; c2=""; c3=""; c4=""; c5=""
  fi
  [ "$USE_UTF8" = 1 ] || on="#"
  # five-row wordmark, letters I G N I S Y E E T
  local r1 r2 r3 r4 r5
  r1="@@@ _@@@ @__@ @@@ _@@@ @___@ @@@@ @@@@ @@@@@"
  r2="_@_ @___ @@_@ _@_ @___ _@_@_ @___ @___ __@__"
  r3="_@_ @_@@ @_@@ _@_ _@@_ __@__ @@@_ @@@_ __@__"
  r4="_@_ @__@ @__@ _@_ ___@ __@__ @___ @___ __@__"
  r5="@@@ _@@@ @__@ @@@ @@@_ __@__ @@@@ @@@@ __@__"
  local rows=("$r1" "$r2" "$r3" "$r4" "$r5") cols=("$c1" "$c2" "$c3" "$c4" "$c5")
  local rocket=("    /\\    " "   /  \\   " "  | () |  " "  |    |  " " /|____|\\ ")
  local rocket_u=("    ╱╲    " "   ╱  ╲   " "  │ ◉◉ │  " "  │    │  " " ╱│▒▒▒▒│╲ ")
  local i line rk
  say ""
  for i in 0 1 2 3 4; do
    line="${rows[$i]}"
    line="${line//@/$on}"
    line="${line//_/$sp}"
    if [ "$USE_UTF8" = 1 ]; then rk="${rocket_u[$i]}"; else rk="${rocket[$i]}"; fi
    say "  ${ORANGE}${rk}${RESET}  ${cols[$i]}${line}${RESET}"
  done
  if [ "$USE_UTF8" = 1 ]; then
    say "  ${YELLOW}    ╲╱╲╱    ${RESET}  ${BOLD}installer${RESET} ${GREY}${G_DOT} rocket flight simulation & aerodynamics toolkit${RESET}"
  else
    say "  ${YELLOW}    VvVv    ${RESET}  ${BOLD}installer${RESET} ${GREY}${G_DOT} rocket flight simulation & aerodynamics toolkit${RESET}"
  fi
  say ""
}

# --- spinner / step runner ---------------------------------------------------------------------
elapsed_since() { # elapsed_since START -> "12s" / "1m05s"
  local d=$(($(date +%s) - $1))
  if [ "$d" -ge 60 ]; then printf '%dm%02ds' $((d / 60)) $((d % 60)); else printf '%ds' "$d"; fi
}

last_log_line() { # last non-empty line of the log (for the spinner's side text)
  tail -n 3 "$LOG_FILE" 2>/dev/null | tr '\r' '\n' | tr -d '\000-\010\013-\037' | sed '/^[[:space:]]*$/d' | tail -n 1 | cut -c1-"$(($1 > 10 ? $1 : 10))"
}

# spin_wait PID LABEL [PROGRESS_FN]: animate until PID exits; returns its exit status.
spin_wait() {
  local pid="$1" label="$2" prog="${3:-}" frames i=0 f extra side
  if [ "$IS_TTY" != 1 ]; then wait "$pid"; return $?; fi
  # shellcheck disable=SC2206
  frames=($SPIN_FRAMES)
  printf '%s' "$HIDE_CUR"
  [ -n "$prog" ] && label="${label:0:$((COLS - 52))}"
  [ -z "$prog" ] && label="${label:0:$((COLS - 30))}"
  while kill -0 "$pid" 2>/dev/null; do
    f="${frames[$((i % ${#frames[@]}))]}"
    if [ -n "$prog" ]; then
      extra=$($prog)
    else
      side=$(last_log_line $((COLS - ${#label} - 16)))
      extra="${GREY}${side}${RESET}"
    fi
    printf '\r%s  %s%s%s %s  %s' "$CLR" "$CYAN" "$f" "$RESET" "$label" "$extra"
    i=$((i + 1))
    sleep 0.1
  done
  printf '\r%s%s' "$CLR" "$SHOW_CUR"
  wait "$pid"
}

# step LABEL CMD...: run a command in the background with a spinner; all output goes to the log.
# Returns the command's status after printing a ✔ / ✖ line with its timing.
step() {
  local label="$1" t0 rc pid
  shift
  t0=$(date +%s)
  logf "step: $label :: $*"
  if [ "$IS_TTY" != 1 ]; then say "  ... $label"; fi
  (
    trap - ERR EXIT
    set +e
    "$@" </dev/null >>"$LOG_FILE" 2>&1
  ) &
  pid=$!
  rc=0
  spin_wait "$pid" "$label" || rc=$?
  step_report "$rc" "$label" "$t0"
  return "$rc"
}

step_report() { # RC LABEL T0
  local t=""
  if [ "$1" -eq 0 ]; then
    [ $(($(date +%s) - $3)) -ge 1 ] && t="  ${GREY}$(elapsed_since "$3")${RESET}"
    say "  ${GREEN}${G_OK}${RESET} $2$t"
    logf "done: $2"
  else
    say "  ${RED}${G_ERR}${RESET} $2  ${RED}failed${RESET}"
    logf "FAILED($1): $2"
    if [ -s "$LOG_FILE" ]; then
      say "${GREY}    ---- last lines of the log ----${RESET}"
      tail -n 8 "$LOG_FILE" | cut -c1-$((COLS - 6)) | sed 's/^/    /'
      say "${GREY}    -------------------------------${RESET}"
    fi
  fi
}

fmt_size() { # bytes -> "3.1 MB"
  awk -v b="$1" 'BEGIN{ if (b>=1048576) printf "%.1f MB", b/1048576; else printf "%.0f kB", b/1024 }'
}

# download URL DEST LABEL: curl in the background with a progress bar.
DL_DEST="" DL_TOTAL=""
dl_progress() {
  local got=0 pct w=18 on off
  [ -f "$DL_DEST" ] && got=$(wc -c <"$DL_DEST" | tr -d ' ')
  if [ -n "$DL_TOTAL" ] && [ "$DL_TOTAL" -gt 0 ] 2>/dev/null; then
    pct=$((got * 100 / DL_TOTAL))
    [ "$pct" -gt 100 ] && pct=100
    on=$((pct * w / 100)); off=$((w - on))
    printf '%s%s%s%s %3d%%  %s / %s' "$CYAN" "$(repeat "$BAR_ON" "$on")" "$GREY" "$(repeat "$BAR_OFF" "$off")$RESET" "$pct" "$(fmt_size "$got")" "$(fmt_size "$DL_TOTAL")"
  else
    printf '%s' "$(fmt_size "$got")"
  fi
}
download() {
  local url="$1" dest="$2" label="$3" t0 rc=0 pid
  DL_DEST="$dest"
  DL_TOTAL=$(curl -fsSIL "$url" 2>>"$LOG_FILE" | tr -d '\r' | awk 'tolower($1)=="content-length:"{v=$2} END{print v}') || DL_TOTAL=""
  t0=$(date +%s)
  logf "download: $url -> $dest ($DL_TOTAL bytes)"
  if [ "$IS_TTY" != 1 ]; then say "  ... $label"; fi
  (
    trap - ERR EXIT
    curl -fsSL --retry 2 -o "$dest" "$url" </dev/null >>"$LOG_FILE" 2>&1
  ) &
  pid=$!
  spin_wait "$pid" "$label" dl_progress || rc=$?
  step_report "$rc" "$label" "$t0"
  return "$rc"
}

# --- prompts -------------------------------------------------------------------------------------
HAVE_TTY=0
detect_tty() {
  if [ "$ASSUME_YES" != 1 ] && ( : </dev/tty ) 2>/dev/null; then HAVE_TTY=1; fi
}

# ask_yn QUESTION DEFAULT(y|n): returns 0 for yes. Non-interactive: the default.
ask_yn() {
  local q="$1" def="$2" hint ans
  if [ "$HAVE_TTY" != 1 ]; then [ "$def" = y ]; return; fi
  if [ "$def" = y ]; then hint="Y/n"; else hint="y/N"; fi
  while :; do
    printf '  %s?%s %s %s[%s]%s ' "$ORANGE" "$RESET" "$q" "$GREY" "$hint" "$RESET" >/dev/tty
    IFS= read -r ans </dev/tty || ans=""
    case "$ans" in
      "") [ "$def" = y ]; return ;;
      [Yy]*) return 0 ;;
      [Nn]*) return 1 ;;
    esac
  done
}

ask_role() {
  [ -n "$ROLE" ] && return
  if [ "$HAVE_TTY" != 1 ]; then ROLE=user; return; fi
  local ans
  say ""
  say "  ${BOLD}How will you use IgnisYeet?${RESET}"
  say "    ${CYAN}1${RESET}) ${BOLD}user${RESET}       prebuilt binary, examples and plotting ${GREY}(fast, small)${RESET}"
  say "    ${CYAN}2${RESET}) ${BOLD}developer${RESET}  git clone, build from source, tests, documentation tooling"
  while :; do
    printf '  %s?%s Choose %s[1/2, default 1]%s ' "$ORANGE" "$RESET" "$GREY" "$RESET" >/dev/tty
    IFS= read -r ans </dev/tty || ans=""
    case "$ans" in
      "" | 1 | u | user) ROLE=user; return ;;
      2 | d | dev | developer) ROLE=developer; return ;;
    esac
  done
}

# ----------------------------------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------------------------------
have() { command -v "$1" >/dev/null 2>&1; }
first_version() { grep -Eo '[0-9]+(\.[0-9]+)+' | head -n 1; }
tool_version() { # tool_version CMD [FLAG]
  "$1" "${2:---version}" </dev/null 2>&1 | head -n 3 | first_version || true
}
die() {
  ERR_REPORTED=1
  say ""
  say "  ${RED}${G_ERR} error:${RESET} $*"
  [ "$LOG_READY" = 1 ] && say "  ${GREY}log: $LOG_FILE${RESET}"
  logf "FATAL: $*"
  exit 1
}

on_err() {
  local rc=$? line="$1"
  [ "$ERR_REPORTED" = 1 ] && exit "$rc"
  ERR_REPORTED=1
  say ""
  say "  ${RED}${G_ERR} unexpected error${RESET} (line $line, status $rc)"
  [ "$LOG_READY" = 1 ] && say "  ${GREY}details: $LOG_FILE${RESET}"
  exit "$rc"
}
on_exit() {
  printf '%s' "${SHOW_CUR:-}"
}
on_int() {
  printf '%s\n' "${SHOW_CUR:-}"
  say "  ${YELLOW}${G_WARN} interrupted${RESET}"
  exit 130
}

normalize_version() { case "$1" in v*) printf '%s' "$1" ;; *) printf 'v%s' "$1" ;; esac; }

sha256_of() {
  if have sha256sum; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi
}

usage() {
  cat <<EOF
IgnisYeet installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | bash -s -- [options]
  bash install.sh [options]

Roles:
  --user               prebuilt binary + examples + plotting environment (default)
  --developer          git clone, build from source, tests, python/doc environments

Options:
  --cfd / --no-cfd     install (or skip) the optional CFD tools: SU2, gmsh, Open MPI (~3 GB, conda-forge)
  --version vX.Y.Z     release to install (default: latest GitHub release); developer: tag to check out
  --prefix DIR         install prefix (default: ~/.local; binary in DIR/bin, data in DIR/share/ignisyeet)
  --dir DIR            developer clone location (default: current clone, else ~/ignisyeet)
  --no-test            developer: skip 'cargo test --release'
  --add-path           append the PATH line to your shell rc file without asking
  --force              reinstall even if the same version is already installed
  -y, --yes            do not ask questions; take the defaults (no CFD, no PATH edit)
  --no-color           plain output (also honours NO_COLOR)
  --dry-run            show what would be done and exit
  --uninstall          remove what this installer installed (asks before optional parts)
  --purge              with --uninstall: also remove the plotting venv and the CFD environment
  -h, --help           this help

Environment variables:
  IGNISYEET_ROLE, IGNISYEET_VERSION, IGNISYEET_PREFIX, IGNISYEET_DIR, IGNISYEET_CFD (1/0),
  IGNISYEET_YES, IGNISYEET_NO_COLOR, IGNISYEET_DRY_RUN, IGNISYEET_FORCE, IGNISYEET_ADD_PATH,
  IGNISYEET_DOWNLOAD_BASE (release download base URL), IGNISYEET_REPO_URL (git URL)

Supported platforms: Linux x86_64 / aarch64, macOS Intel / Apple Silicon.
EOF
}

parse_args() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --user) ROLE=user ;;
      --developer | --dev) ROLE=developer ;;
      --cfd) WANT_CFD=1 ;;
      --no-cfd) WANT_CFD=0 ;;
      --version) shift; [ $# -gt 0 ] || die "--version needs a value"; VERSION="$1" ;;
      --version=*) VERSION="${1#*=}" ;;
      --prefix) shift; [ $# -gt 0 ] || die "--prefix needs a value"; PREFIX="$1" ;;
      --prefix=*) PREFIX="${1#*=}" ;;
      --dir) shift; [ $# -gt 0 ] || die "--dir needs a value"; DEV_DIR="$1" ;;
      --dir=*) DEV_DIR="${1#*=}" ;;
      --no-test) WANT_TEST=0 ;;
      --add-path) ADD_PATH=1 ;;
      --force) FORCE=1 ;;
      -y | --yes) ASSUME_YES=1 ;;
      --no-color) NO_COLOR_FLAG=1 ;;
      --dry-run) DRY_RUN=1 ;;
      --uninstall) ACTION=uninstall ;;
      --purge) PURGE=1 ;;
      -h | --help) HELP=1 ;;
      *) die "unknown option: $1 (see --help)" ;;
    esac
    shift
  done
  case "$ROLE" in "" | user | developer) ;; *) die "role must be 'user' or 'developer' (got '$ROLE')" ;; esac
  case "$WANT_CFD" in "" | 0 | 1) ;; *) die "IGNISYEET_CFD must be 1 or 0" ;; esac
  [ -n "$VERSION" ] && VERSION=$(normalize_version "$VERSION")
  return 0
}

# ----------------------------------------------------------------------------------------------
# Platform and tool detection
# ----------------------------------------------------------------------------------------------
detect_platform() {
  OS=$(uname -s)
  ARCH=$(uname -m)
  case "$OS/$ARCH" in
    Linux/x86_64 | Linux/amd64) TARGET=x86_64-unknown-linux-musl; MM_PLAT=linux-64; OS_NAME="Linux x86_64" ;;
    Linux/aarch64 | Linux/arm64) TARGET=aarch64-unknown-linux-musl; MM_PLAT=linux-aarch64; OS_NAME="Linux aarch64" ;;
    Darwin/x86_64)
      if [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = 1 ]; then
        TARGET=aarch64-apple-darwin; MM_PLAT=osx-arm64; OS_NAME="macOS Apple Silicon (Rosetta shell)"
      else
        TARGET=x86_64-apple-darwin; MM_PLAT=osx-64; OS_NAME="macOS Intel"
      fi ;;
    Darwin/arm64) TARGET=aarch64-apple-darwin; MM_PLAT=osx-arm64; OS_NAME="macOS Apple Silicon" ;;
    *) die "unsupported platform: $OS/$ARCH (supported: Linux x86_64/aarch64, macOS Intel/Apple Silicon)" ;;
  esac
  [ -n "${IGNISYEET_TARGET:-}" ] && TARGET="$IGNISYEET_TARGET"
  return 0
}

cpu_has_avx512() {
  [ "$OS" = Linux ] && [ "$(uname -m)" = x86_64 ] && grep -qw avx512f /proc/cpuinfo 2>/dev/null
}

# Locate an existing conda-forge environment named ignisyeet-cfd under any conda root.
find_cfd_env() {
  local d roots="" r
  [ -n "${IGNISYEET_CFD_PREFIX:-}" ] && [ -x "$IGNISYEET_CFD_PREFIX/bin/SU2_CFD" ] && { printf '%s' "$IGNISYEET_CFD_PREFIX"; return 0; }
  [ -n "${MAMBA_ROOT_PREFIX:-}" ] && roots="$MAMBA_ROOT_PREFIX"
  [ -n "${CONDA_PREFIX:-}" ] && roots="$roots ${CONDA_PREFIX%%/envs/*}"
  for r in "$HOME/.local/share/micromamba" "$HOME/micromamba" "$HOME/.local/share/mamba" "$HOME/miniforge3" "$HOME/mambaforge" \
    "$HOME/miniconda3" "$HOME/anaconda3" "$HOME/.conda" /opt/conda /opt/miniforge3 /opt/homebrew/Caskroom/miniforge/base \
    /usr/local/Caskroom/miniforge/base; do
    roots="$roots $r"
  done
  for r in $roots; do
    d="$r/envs/$CFD_ENV_NAME"
    if [ -x "$d/bin/SU2_CFD" ]; then printf '%s' "$d"; return 0; fi
  done
  return 1
}

find_conda_manager() { # prints path of micromamba / mamba / conda
  local c
  for c in micromamba mamba conda; do
    if have "$c"; then command -v "$c"; return 0; fi
  done
  for c in "$BIN_DIR/micromamba" "$HOME/.local/bin/micromamba" "$HOME/micromamba/bin/micromamba" "$HOME/miniforge3/bin/mamba" \
    "$HOME/mambaforge/bin/mamba" "$HOME/miniconda3/bin/conda" "$HOME/anaconda3/bin/conda" /opt/conda/bin/conda; do
    [ -x "$c" ] && { printf '%s' "$c"; return 0; }
  done
  return 1
}

conda_pkg_version() { # ENV PKG
  local f
  f=""
  for f in "$1/conda-meta/$2"-[0-9]*.json; do break; done
  [ -f "$f" ] || return 0
  basename "$f" | sed -E "s/^$2-([0-9][^-]*)-.*/\\1/"
}

# Detect everything once; results go into HAVE_* / *_VER variables.
detect_tools() {
  HAVE_CURL=0; HAVE_TAR=0
  have curl && HAVE_CURL=1
  have tar && HAVE_TAR=1
  GIT_VER=""; have git && GIT_VER=$(tool_version git)

  # ignisyeet itself
  IGN_PATH=""; IGN_VER=""
  if [ -x "$BIN_DIR/ignisyeet" ]; then IGN_PATH="$BIN_DIR/ignisyeet"; elif have ignisyeet; then IGN_PATH=$(command -v ignisyeet); fi
  [ -n "$IGN_PATH" ] && IGN_VER=$("$IGN_PATH" --version </dev/null 2>&1 | first_version || true)

  # Rust
  CARGO_PATH=""; CARGO_VER=""
  if have cargo; then CARGO_PATH=$(command -v cargo); elif [ -x "$HOME/.cargo/bin/cargo" ]; then CARGO_PATH="$HOME/.cargo/bin/cargo"; fi
  [ -n "$CARGO_PATH" ] && CARGO_VER=$("$CARGO_PATH" --version </dev/null 2>&1 | first_version || true)

  # uv and python
  UV_PATH=""; UV_VER=""
  if have uv; then UV_PATH=$(command -v uv); elif [ -x "$HOME/.local/bin/uv" ]; then UV_PATH="$HOME/.local/bin/uv"; fi
  [ -n "$UV_PATH" ] && UV_VER=$("$UV_PATH" --version </dev/null 2>&1 | first_version || true)
  PY_PATH=""; PY_VER=""; PY_OK=0
  if have python3; then
    PY_PATH=$(command -v python3)
    PY_VER=$(python3 --version 2>&1 | first_version || true)
    python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null && PY_OK=1
  fi

  # CFD stack
  CONDA_MGR=""; CONDA_VER=""
  CONDA_MGR=$(find_conda_manager || true)
  [ -n "$CONDA_MGR" ] && CONDA_VER=$("$CONDA_MGR" --version </dev/null 2>&1 | first_version || true)
  CFD_PREFIX=$(find_cfd_env || true)
  CFD_SU2_VER=""; CFD_SU2_BAD=0
  SU2_PATH=""; GMSH_PATH=""; MPI_PATH=""
  if [ -n "$CFD_PREFIX" ]; then
    CFD_SU2_VER=$(conda_pkg_version "$CFD_PREFIX" su2)
    case "$CFD_SU2_VER" in 8.[4-9]* | 9*) cpu_has_avx512 || { [ "$OS" = Linux ] && [ "$ARCH" = x86_64 ] && CFD_SU2_BAD=1; } ;; esac
  fi
  have SU2_CFD && SU2_PATH=$(command -v SU2_CFD)
  have gmsh && GMSH_PATH=$(command -v gmsh)
  have mpirun && MPI_PATH=$(command -v mpirun)

  # documentation tooling (developer)
  DOC_MISSING_APT=""; DOC_MISSING_BREW=""; DOC_MISSING_NAMES=""
  doc_check lualatex "lualatex (LuaTeX)" texlive-luatex "mactex-no-gui"
  if have kpsewhich; then
    kpsewhich luatexja.sty >/dev/null 2>&1 || doc_add "luatexja" texlive-lang-japanese "mactex-no-gui"
  fi
  doc_check uplatex "uplatex" texlive-lang-japanese "mactex-no-gui"
  doc_check dvisvgm "dvisvgm" dvisvgm "mactex-no-gui"
  doc_check mutool "mutool (mupdf-tools)" mupdf-tools mupdf-tools
  doc_check inkscape "inkscape" inkscape "--cask inkscape"
  if ! fonts_ok; then doc_add "IPAex fonts" fonts-ipaexfont "--cask font-ipaexfont"; fi
}

fonts_ok() {
  if have fc-list; then fc-list 2>/dev/null | grep -qi 'ipaex'; return; fi
  find /Library/Fonts /usr/share/fonts "$HOME/Library/Fonts" -iname '*ipaex*' 2>/dev/null | grep -q .
}
doc_add() { # NAME APT BREW
  DOC_MISSING_NAMES="${DOC_MISSING_NAMES:+$DOC_MISSING_NAMES, }$1"
  case " $DOC_MISSING_APT " in *" $2 "*) ;; *) DOC_MISSING_APT="${DOC_MISSING_APT:+$DOC_MISSING_APT }$2" ;; esac
  case " $DOC_MISSING_BREW " in *" $3 "*) ;; *) DOC_MISSING_BREW="${DOC_MISSING_BREW:+$DOC_MISSING_BREW }$3" ;; esac
}
doc_check() { # CMD NAME APT BREW
  have "$1" || doc_add "$2" "$3" "$4"
  return 0
}

# ----------------------------------------------------------------------------------------------
# Reporting rows
# ----------------------------------------------------------------------------------------------
row_found() { say "  ${GREEN}${G_OK}${RESET} $(printf '%-13s' "$1") ${GREY}$2${RESET}"; }
row_miss()  { say "  ${YELLOW}${G_SKIP}${RESET} $(printf '%-13s' "$1") ${YELLOW}$2${RESET}"; }

show_environment() {
  section "Environment"
  say "  ${GREY}platform${RESET}      $OS_NAME ${GREY}($TARGET)${RESET}"
  say "  ${GREY}prefix${RESET}        $(tilde "$PREFIX")"
  if [ -n "$IGN_PATH" ]; then row_found ignisyeet "${IGN_VER:-?}  $(tilde "$IGN_PATH")"; else row_miss ignisyeet "not installed"; fi
  if [ "$HAVE_CURL" = 1 ]; then row_found curl "$(tool_version curl)"; else row_miss curl "missing (required)"; fi
  if [ "$HAVE_TAR" = 1 ]; then row_found tar "present"; else row_miss tar "missing (required)"; fi
  if [ -n "$GIT_VER" ]; then row_found git "$GIT_VER"; else row_miss git "not found$( [ "$ROLE" = developer ] && echo ' (required)')"; fi
  if [ -n "$UV_PATH" ]; then row_found uv "$UV_VER  $(tilde "$UV_PATH")"; else row_miss uv "not found -> will offer to install"; fi
  if [ "$PY_OK" = 1 ]; then row_found python3 "$PY_VER  $(tilde "$PY_PATH")"
  elif [ -n "$PY_PATH" ]; then row_miss python3 "$PY_VER is older than 3.10$( [ -n "$UV_PATH" ] && echo ' (uv can provide one)')"
  else row_miss python3 "not found$( [ -n "$UV_PATH" ] && echo ' (uv can provide one)')"; fi
  if [ "$ROLE" = developer ]; then
    if [ -n "$CARGO_PATH" ]; then row_found cargo "$CARGO_VER  $(tilde "$CARGO_PATH")"; else row_miss cargo "not found -> rustup will be installed"; fi
  fi
  if [ -n "$CFD_PREFIX" ]; then
    row_found "CFD env" "$CFD_ENV_NAME  su2 ${CFD_SU2_VER:-?}  $(tilde "$CFD_PREFIX")"
    [ "$CFD_SU2_BAD" = 1 ] && warn "su2 $CFD_SU2_VER needs AVX-512, which this CPU lacks (SU2_CFD will crash); recreate the env with su2=8.3.0"
  elif [ -n "$SU2_PATH" ]; then
    row_found "SU2_CFD" "$SU2_PATH"
  else
    row_miss "CFD tools" "SU2 / gmsh / mpirun not found (optional)"
  fi
  [ -n "$CONDA_MGR" ] && row_found "conda" "$(basename "$CONDA_MGR") ${CONDA_VER:-?}  $(tilde "$CONDA_MGR")"
  [ -n "$GMSH_PATH" ] && [ -z "$CFD_PREFIX" ] && row_found gmsh "$GMSH_PATH"
  [ -n "$MPI_PATH" ] && [ -z "$CFD_PREFIX" ] && row_found mpirun "$MPI_PATH"
  return 0
}

# ----------------------------------------------------------------------------------------------
# Version resolution
# ----------------------------------------------------------------------------------------------
resolve_latest_version() {
  local tag=""
  tag=$(curl -fsSL -H 'Accept: application/vnd.github+json' "https://api.github.com/repos/$REPO/releases/latest" 2>>"$LOG_FILE" |
    sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1) || tag=""
  if [ -z "$tag" ]; then
    local url
    url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" 2>>"$LOG_FILE") || url=""
    case "$url" in */releases/tag/*) tag="${url##*/}" ;; esac
  fi
  printf '%s' "$tag"
}

# ----------------------------------------------------------------------------------------------
# Paths
# ----------------------------------------------------------------------------------------------
init_paths() {
  BIN_DIR="$PREFIX/bin"
  DATA_ROOT="$PREFIX/share/ignisyeet"
  LOG_FILE="$DATA_ROOT/install.log"
  STATE_FILE="$DATA_ROOT/state"
  VENV_DIR="$DATA_ROOT/venv"
  if [ "$DRY_RUN" = 1 ]; then LOG_FILE=/dev/null; return 0; fi
  mkdir -p "$DATA_ROOT" 2>/dev/null || die "cannot create $DATA_ROOT"
  : >>"$LOG_FILE" || die "cannot write $LOG_FILE"
  LOG_READY=1
  printf '\n===== install.sh %s  %s  args: %s =====\n' "$(date '+%F %T')" "$(uname -a)" "${ORIG_ARGS:-}" >>"$LOG_FILE"
}

state_get() { [ -f "$STATE_FILE" ] && sed -n "s/^$1=//p" "$STATE_FILE" | tail -n 1 || true; }
state_set() { # KEY VALUE
  local tmp="$STATE_FILE.tmp"
  { [ -f "$STATE_FILE" ] && grep -v "^$1=" "$STATE_FILE" || true; printf '%s=%s\n' "$1" "$2"; } >"$tmp"
  mv "$tmp" "$STATE_FILE"
}

# ----------------------------------------------------------------------------------------------
# Decisions (no side effects)
# ----------------------------------------------------------------------------------------------
decide() {
  # developer clone location
  if [ "$ROLE" = developer ] && [ -z "$DEV_DIR" ]; then
    local top
    top=$(git rev-parse --show-toplevel 2>/dev/null || true)
    if [ -n "$top" ] && [ -f "$top/crates/cli/Cargo.toml" ] && grep -q 'name = "ignisyeet"' "$top/crates/cli/Cargo.toml" 2>/dev/null; then
      DEV_DIR="$top"
    else
      DEV_DIR="$HOME/ignisyeet"
    fi
  fi

  # release version (user role: required; developer: only when pinned)
  if [ "$ROLE" = user ] && [ -z "$VERSION" ]; then
    if [ -n "${IGNISYEET_DOWNLOAD_BASE:-}" ]; then
      die "IGNISYEET_DOWNLOAD_BASE is set; also give --version vX.Y.Z"
    fi
    VERSION=$(resolve_latest_version)
    if [ -z "$VERSION" ]; then
      if [ "$DRY_RUN" = 1 ]; then VERSION="(latest)"; else
        die "could not determine the latest release of $REPO (no release published yet, or no network). Use --version vX.Y.Z or --developer."
      fi
    fi
  fi

  # uv
  USE_UV=0; INSTALL_UV=0
  if [ -n "$UV_PATH" ]; then USE_UV=1
  elif [ "$PY_OK" != 1 ] || [ "$ROLE" = developer ]; then
    # python missing or old: uv is the easiest way to get one; developers need it for the doc env
    if ask_yn "uv (fast Python manager) is not installed. Install it from astral.sh?" y; then USE_UV=1; INSTALL_UV=1; fi
  else
    if ask_yn "uv (fast Python manager) is not installed. Install it from astral.sh? (No: use python3 -m venv + pip)" y; then USE_UV=1; INSTALL_UV=1; fi
  fi
  if [ "$USE_UV" = 0 ] && [ "$PY_OK" != 1 ]; then
    die "Python >= 3.10 is required for plotting; install python3 (>= 3.10) or allow uv to be installed."
  fi

  # rust
  INSTALL_RUST=0
  [ "$ROLE" = developer ] && [ -z "$CARGO_PATH" ] && INSTALL_RUST=1

  # tests
  if [ "$ROLE" = developer ] && [ -z "$WANT_TEST" ]; then
    if ask_yn "Run 'cargo test --release' after building?" y; then WANT_TEST=1; else WANT_TEST=0; fi
  fi

  # CFD
  CFD_DISABLED=0
  [ "$WANT_CFD" = 0 ] && CFD_DISABLED=1
  if [ -z "$WANT_CFD" ]; then
    if [ -n "$CFD_PREFIX" ] || [ -n "$SU2_PATH" ]; then
      WANT_CFD=0 # already available; nothing to ask
    elif ask_yn "Install the optional CFD tools (SU2, gmsh, Open MPI via conda-forge, ~3 GB)?" n; then WANT_CFD=1; else WANT_CFD=0; fi
  fi
  CFD_ACTION=none # none | reuse | create
  if [ "$CFD_DISABLED" = 1 ]; then
    CFD_ACTION=none
  elif [ -n "$CFD_PREFIX" ]; then
    CFD_ACTION=reuse
  elif [ "$WANT_CFD" = 1 ]; then
    if [ -n "$SU2_PATH" ] && [ -n "$GMSH_PATH" ] && [ -n "$MPI_PATH" ] && [ "$FORCE" != 1 ]; then CFD_ACTION=reuse; else CFD_ACTION=create; fi
  fi
  SU2_PIN=""
  if [ "$CFD_ACTION" = create ] && [ "$OS" = Linux ] && [ "$ARCH" = x86_64 ] && ! cpu_has_avx512; then SU2_PIN="su2=8.3.0"; fi
  CFD_NEED_MM=0
  [ "$CFD_ACTION" = create ] && [ -z "$CONDA_MGR" ] && CFD_NEED_MM=1

  # user: already installed?
  SAME_VERSION=0
  if [ "$ROLE" = user ] && [ -n "$IGN_VER" ] && [ "v$IGN_VER" = "$VERSION" ] && [ -d "$DATA_ROOT/$VERSION" ] && [ "$FORCE" != 1 ]; then
    SAME_VERSION=1
    if [ "$HAVE_TTY" = 1 ] && ask_yn "ignisyeet $VERSION is already installed. Reinstall it?" n; then SAME_VERSION=0; fi
  fi
}

show_plan() {
  section "Plan"
  local n=1
  p() { say "  ${BOLD}$n.${RESET} $*"; n=$((n + 1)); }
  say "  ${GREY}role${RESET}         ${BOLD}$ROLE${RESET}"
  if [ "$ROLE" = user ]; then
    say "  ${GREY}version${RESET}      $VERSION"
    if [ "$SAME_VERSION" = 1 ]; then p "keep ignisyeet $VERSION ${GREY}(already installed)${RESET}"
    else p "download ${CYAN}ignisyeet-$VERSION-$TARGET.tar.gz${RESET}, verify sha256, install to ${CYAN}$DATA_ROOT/$VERSION${RESET}"
      p "link ${CYAN}$BIN_DIR/ignisyeet${RESET} and ${CYAN}ignisyeet-plot${RESET}"; fi
    [ "$INSTALL_UV" = 1 ] && p "install ${CYAN}uv${RESET} (astral.sh installer)"
    if [ "$USE_UV" = 1 ]; then p "plotting env (numpy, pandas, matplotlib) with uv ${GREY}${UV_VER:+(reusing uv $UV_VER)}${RESET} in $VENV_DIR"
    else p "plotting env with python3 -m venv + pip in $VENV_DIR"; fi
  else
    say "  ${GREY}directory${RESET}    $DEV_DIR"
    if [ -d "$DEV_DIR/.git" ]; then p "reuse clone ${CYAN}$DEV_DIR${RESET}; ${GREY}git pull --ff-only if clean${RESET}"
    else p "git clone ${CYAN}$REPO_URL${RESET} -> $DEV_DIR"; fi
    [ -n "$VERSION" ] && p "check out ${CYAN}$VERSION${RESET}"
    if [ "$INSTALL_RUST" = 1 ]; then p "install ${CYAN}Rust${RESET} via rustup (sh.rustup.rs, minimal profile)"
    else p "reuse ${CYAN}cargo $CARGO_VER${RESET}"; fi
    [ "$INSTALL_UV" = 1 ] && p "install ${CYAN}uv${RESET} (astral.sh installer)"
    p "cargo build --release"
    [ "$WANT_TEST" = 1 ] && p "cargo test --release"
    if [ "$USE_UV" = 1 ]; then p "uv sync: python/ (plotting) and doc/ (Sphinx) environments"
    else p "venv + pip: .venv with python/ and doc/ requirements"; fi
    if [ -n "$DOC_MISSING_NAMES" ]; then p "documentation tools missing (${DOC_MISSING_NAMES}); the exact install command is printed at the end"
    else p "documentation tools: ${GREEN}all found${RESET}"; fi
  fi
  case "$CFD_ACTION" in
    reuse) p "CFD: ${GREEN}reuse${RESET} ${CFD_PREFIX:-$SU2_PATH}" ;;
    create)
      [ "$CFD_NEED_MM" = 1 ] && p "install ${CYAN}micromamba${RESET} to $BIN_DIR/micromamba"
      p "CFD: create conda-forge env ${CYAN}$CFD_ENV_NAME${RESET} with ${CFD_PACKAGES/su2/${SU2_PIN:-su2}} ${GREY}(~3 GB)${RESET}"
      [ -n "$SU2_PIN" ] && note "no AVX-512 on this CPU: su2 is pinned to 8.3.0 (8.5.0 crashes with SIGILL)" ;;
    none) skip "CFD: not requested" ;;
  esac
}

# ----------------------------------------------------------------------------------------------
# Actions
# ----------------------------------------------------------------------------------------------
ensure_uv() {
  [ "$INSTALL_UV" = 1 ] || return 0
  step "Installing uv" sh -c 'curl -LsSf https://astral.sh/uv/install.sh | sh' || die "uv installation failed"
  UV_PATH=""
  for p in "$HOME/.local/bin/uv" "$HOME/.cargo/bin/uv"; do [ -x "$p" ] && UV_PATH="$p" && break; done
  [ -n "$UV_PATH" ] || die "uv was installed but cannot be found"
  UV_VER=$("$UV_PATH" --version | first_version || true)
}

# Create a venv at DIR with the requirements file REQ (uv or venv+pip).
make_venv() { # DIR REQ LABEL
  local dir="$1" req="$2" label="$3"
  if [ -x "$dir/bin/python" ] && "$dir/bin/python" -c 'import numpy, pandas, matplotlib' >/dev/null 2>&1; then
    row_found "$label" "already set up ($dir)"
    return 0
  fi
  if [ "$USE_UV" = 1 ]; then
    step "$label: creating environment (uv)" "$UV_PATH" venv --quiet --allow-existing --python '>=3.10' "$dir" || die "uv venv failed"
    step "$label: installing numpy, pandas, matplotlib" "$UV_PATH" pip install --quiet --python "$dir/bin/python" -r "$req" || die "uv pip install failed"
  else
    step "$label: creating environment (venv)" python3 -m venv "$dir" || die "python3 -m venv failed (on Debian/Ubuntu: apt install python3-venv)"
    step "$label: installing numpy, pandas, matplotlib" "$dir/bin/python" -m pip install --quiet -r "$req" || die "pip install failed"
  fi
}

install_user() {
  section "Installing ignisyeet $VERSION"
  [ "$HAVE_CURL" = 1 ] && [ "$HAVE_TAR" = 1 ] || die "curl and tar are required"
  have sha256sum || have shasum || die "sha256sum or shasum is required to verify the download"
  TMP=$(mktemp -d "${TMPDIR:-/tmp}/ignisyeet-install.XXXXXX")
  trap 'rm -rf "$TMP"; printf "%s" "${SHOW_CUR:-}"' EXIT
  local name="ignisyeet-$VERSION-$TARGET" url
  url="$DOWNLOAD_BASE/$VERSION/$name.tar.gz"
  if [ "$SAME_VERSION" = 1 ]; then
    row_found "ignisyeet" "$VERSION already installed; skipping download"
  else
    download "$url" "$TMP/$name.tar.gz" "Downloading ignisyeet $VERSION" ||
      die "download failed: $url (does release $VERSION exist for $TARGET?)"
    download "$url.sha256" "$TMP/$name.tar.gz.sha256" "Downloading checksum" || die "checksum download failed"
    local want got
    want=$(awk '{print $1; exit}' "$TMP/$name.tar.gz.sha256")
    got=$(sha256_of "$TMP/$name.tar.gz")
    if [ -z "$want" ] || [ "$want" != "$got" ]; then
      say "  ${RED}${G_ERR}${RESET} checksum mismatch"
      die "sha256 mismatch for $name.tar.gz (expected $want, got $got); refusing to install"
    fi
    ok "sha256 verified ${GREY}${got:0:16}...${RESET}"
    step "Extracting" tar -xzf "$TMP/$name.tar.gz" -C "$TMP" || die "extraction failed"
    [ -x "$TMP/$name/bin/ignisyeet" ] || die "archive has an unexpected layout (no $name/bin/ignisyeet)"
    step "Installing files" sh -c '
      set -e
      rm -rf "$2/$3.new" "$2/$3"
      mkdir -p "$1" "$2"
      cp -R "$4" "$2/$3.new"
      mv "$2/$3.new" "$2/$3"
      ln -sfn "$2/$3" "$2/current"
      ln -sfn "$2/current/bin/ignisyeet" "$1/ignisyeet"
    ' sh "$BIN_DIR" "$DATA_ROOT" "$VERSION" "$TMP/$name" || die "installation failed"
  fi
  write_plot_wrapper
  ok "linked ${CYAN}$BIN_DIR/ignisyeet${RESET}"
  state_set version "$VERSION"
  state_set role user

  section "Plotting environment"
  [ "$INSTALL_UV" = 1 ] && ensure_uv
  make_venv "$VENV_DIR" "$DATA_ROOT/current/python/requirements.txt" "plotting"
  state_set venv "$VENV_DIR"
}

write_plot_wrapper() {
  mkdir -p "$BIN_DIR"
  cat >"$BIN_DIR/ignisyeet-plot" <<EOF
#!/bin/sh
# Generated by the IgnisYeet installer: runs python/plot.py inside the plotting environment.
exec "$VENV_DIR/bin/python" "$DATA_ROOT/current/python/plot.py" "\$@"
EOF
  chmod +x "$BIN_DIR/ignisyeet-plot"
}

ensure_rust() {
  if [ "$INSTALL_RUST" = 1 ]; then
    step "Installing Rust (rustup, minimal profile)" sh -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal' ||
      die "rustup installation failed"
  fi
  # shellcheck disable=SC1091
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  have cargo || die "cargo not found after installing Rust"
  CARGO_PATH=$(command -v cargo)
  CARGO_VER=$(cargo --version | first_version || true)
}

install_developer() {
  [ -n "$GIT_VER" ] || die "git is required for the developer install (apt install git / xcode-select --install)"
  section "Source tree"
  local dirty
  if [ -d "$DEV_DIR/.git" ]; then
    ok "reusing clone ${CYAN}$DEV_DIR${RESET}"
    dirty=$(git -C "$DEV_DIR" status --porcelain 2>/dev/null | head -n 1 || true)
    if [ -n "$dirty" ]; then
      warn "working tree has local changes; not updating it"
    elif git -C "$DEV_DIR" rev-parse --abbrev-ref '@{upstream}' >/dev/null 2>&1; then
      step "git pull --ff-only" git -C "$DEV_DIR" pull --ff-only || warn "git pull failed; continuing with the current checkout"
    fi
  else
    if [ -e "$DEV_DIR" ] && [ -n "$(ls -A "$DEV_DIR" 2>/dev/null)" ]; then die "$DEV_DIR exists and is not an IgnisYeet clone"; fi
    step "git clone $REPO" git clone "$REPO_URL" "$DEV_DIR" || die "git clone failed"
  fi
  if [ -n "$VERSION" ]; then
    step "Checking out $VERSION" sh -c 'git -C "$1" fetch --tags --quiet 2>/dev/null; git -C "$1" checkout --quiet "$2"' sh "$DEV_DIR" "$VERSION" ||
      die "cannot check out $VERSION"
  fi
  state_set role developer
  state_set dev_dir "$DEV_DIR"

  section "Toolchain"
  [ "$INSTALL_UV" = 1 ] && ensure_uv
  ensure_rust
  [ "$INSTALL_RUST" = 0 ] && row_found cargo "$CARGO_VER (reused)"

  section "Build"
  step "cargo build --release" sh -c 'cd "$1" && cargo build --release' sh "$DEV_DIR" || die "build failed"
  ok "binary: ${CYAN}$DEV_DIR/target/release/ignisyeet${RESET}"
  if [ "$WANT_TEST" = 1 ]; then
    step "cargo test --release" sh -c 'cd "$1" && cargo test --release' sh "$DEV_DIR" || die "tests failed (see the log)"
  fi

  section "Python environments"
  if [ "$USE_UV" = 1 ]; then
    step "uv sync (python/ plotting)" "$UV_PATH" sync --quiet --project "$DEV_DIR/python" || die "uv sync python failed"
    step "uv sync (doc/ Sphinx)" "$UV_PATH" sync --quiet --project "$DEV_DIR/doc" || die "uv sync doc failed"
  else
    make_venv "$DEV_DIR/.venv" "$DEV_DIR/python/requirements.txt" "plotting"
    [ -f "$DEV_DIR/doc/requirements.txt" ] &&
      { step "doc requirements (pip)" "$DEV_DIR/.venv/bin/python" -m pip install --quiet -r "$DEV_DIR/doc/requirements.txt" || warn "doc requirements failed"; }
  fi

  section "Documentation tools"
  if [ -z "$DOC_MISSING_NAMES" ]; then
    ok "TeX Live (luatexja, uplatex), dvisvgm, mutool, inkscape, IPAex fonts: all found"
  else
    warn "missing: ${DOC_MISSING_NAMES}"
    if [ "$OS" = Darwin ]; then
      say "    ${GREY}install with Homebrew:${RESET}"
      say "      ${CYAN}brew install $(printf '%s' "$DOC_MISSING_BREW" | sed 's/mactex-no-gui/--cask mactex-no-gui/')${RESET}"
    else
      say "    ${GREY}install with apt (needs sudo; not run by this installer):${RESET}"
      say "      ${CYAN}sudo apt-get install -y --no-install-recommends $DOC_MISSING_APT${RESET}"
    fi
  fi
}

# --- CFD ---------------------------------------------------------------------------------------
install_cfd() {
  if [ "$CFD_ACTION" = none ]; then return 0; fi
  section "CFD tools (SU2, gmsh, Open MPI)"
  case "$CFD_ACTION" in
    reuse)
      if [ -n "$CFD_PREFIX" ]; then
        ok "reusing existing environment ${CYAN}$CFD_PREFIX${RESET} ${GREY}(su2 ${CFD_SU2_VER:-?})${RESET}"
        [ "$CFD_SU2_BAD" = 1 ] && warn "su2 $CFD_SU2_VER needs AVX-512; recreate with: $CONDA_MGR create -y -n $CFD_ENV_NAME -c conda-forge --override-channels $CFD_PACKAGES su2=8.3.0"
      else
        ok "reusing SU2/gmsh/mpirun found in PATH"
        CFD_PREFIX=$(dirname "$(dirname "$SU2_PATH")")
      fi
      return 0 ;;
  esac
  local mm="$CONDA_MGR" root_args=""
  if [ "$CFD_NEED_MM" = 1 ]; then
    mm="$BIN_DIR/micromamba"
    [ -z "${MAMBA_ROOT_PREFIX:-}" ] && root_args="-r $HOME/.local/share/micromamba"
    mkdir -p "$BIN_DIR"
    step "Installing micromamba ($MM_PLAT)" sh -c '
      set -e
      d=$(mktemp -d); trap "rm -rf $d" EXIT
      curl -fsSL "https://micro.mamba.pm/api/micromamba/$1/latest" | tar -xj -C "$d" bin/micromamba
      install -m 755 "$d/bin/micromamba" "$2"
    ' sh "$MM_PLAT" "$mm" || { warn "micromamba installation failed; CFD skipped"; return 0; }
    state_set micromamba "$mm"
  else
    ok "reusing $(basename "$mm") ${GREY}$mm${RESET}"
  fi
  local pkgs="$CFD_PACKAGES"
  [ -n "$SU2_PIN" ] && pkgs="${pkgs/su2/$SU2_PIN}"
  note "this downloads about 3 GB; progress is in $LOG_FILE"
  # shellcheck disable=SC2086
  if ! step "Creating conda env $CFD_ENV_NAME (conda-forge)" "$mm" $root_args create -y -n "$CFD_ENV_NAME" -c conda-forge --override-channels $pkgs; then
    warn "CFD environment could not be created (SU2 may be unavailable for $MM_PLAT); CFD skipped"
    return 0
  fi
  CFD_PREFIX=$(find_cfd_env || true)
  if [ -z "$CFD_PREFIX" ] && [ -n "$root_args" ] && [ -x "$HOME/.local/share/micromamba/envs/$CFD_ENV_NAME/bin/SU2_CFD" ]; then
    CFD_PREFIX="$HOME/.local/share/micromamba/envs/$CFD_ENV_NAME"
  fi
  if [ -z "$CFD_PREFIX" ]; then warn "environment created but SU2_CFD was not found in it"; return 0; fi
  CFD_SU2_VER=$(conda_pkg_version "$CFD_PREFIX" su2)
  ok "SU2 ${CFD_SU2_VER:-?} installed in ${CYAN}$CFD_PREFIX${RESET}"
  state_set cfd_env "$CFD_PREFIX"
}

# --- PATH hint -------------------------------------------------------------------------------------
shell_rc() {
  case "$(basename "${SHELL:-}")" in
    zsh) echo "$HOME/.zshrc" ;;
    bash) if [ "$OS" = Darwin ]; then echo "$HOME/.bash_profile"; else echo "$HOME/.bashrc"; fi ;;
    *) echo "" ;;
  esac
}
path_hint() {
  case ":$PATH:" in *":$BIN_DIR:"*) return 0 ;; esac
  PATH_MISSING=1
  local rc line="export PATH=\"$BIN_DIR:\$PATH\""
  rc=$(shell_rc)
  if [ -n "$rc" ] && ! grep -qsF "$line" "$rc"; then
    if [ "$ADD_PATH" = 1 ] || ask_yn "Add $BIN_DIR to PATH in $rc?" n; then
      printf '\n# Added by the IgnisYeet installer\n%s\n' "$line" >>"$rc"
      ok "appended the PATH line to ${CYAN}$rc${RESET}"
      PATH_MISSING=2
    fi
  fi
}

# --- summary ---------------------------------------------------------------------------------------
summary() {
  local took
  took=$(elapsed_since "$START_TIME")
  say ""
  box_top "${GREEN}${G_OK}${RESET} ${BOLD}IgnisYeet is ready${RESET} ${GREY}($took)${RESET}"
  box_row ""
  if [ "$ROLE" = user ]; then
    box_row "${BOLD}version${RESET}   $VERSION"
    box_row "${BOLD}binary${RESET}    $(tilde "$BIN_DIR")/ignisyeet"
    box_row "${BOLD}data${RESET}      $(tilde "$DATA_ROOT")/$VERSION"
    box_row "${BOLD}plotting${RESET}  ignisyeet-plot  ${GREY}(env: $(tilde "$VENV_DIR"))${RESET}"
  else
    box_row "${BOLD}clone${RESET}     $(tilde "$DEV_DIR")"
    box_row "${BOLD}binary${RESET}    $(tilde "$DEV_DIR")/target/release/ignisyeet"
    box_row "${BOLD}cargo${RESET}     ${CARGO_VER:-?}  ${GREY}tests: $( [ "$WANT_TEST" = 1 ] && echo passed || echo skipped )${RESET}"
  fi
  if [ -n "$CFD_PREFIX" ] && [ "$CFD_ACTION" != none ]; then box_row "${BOLD}CFD${RESET}       $(tilde "$CFD_PREFIX")"; fi
  box_row ""
  box_row "${BOLD}Next${RESET}"
  if [ "$ROLE" = user ]; then
    local ex; ex="$(cmdpath "$DATA_ROOT")/current/examples"
    [ "${PATH_MISSING:-0}" = 1 ] && box_row "  ${CYAN}export PATH=\"$(cmdpath "$BIN_DIR"):\$PATH\"${RESET}"
    box_row "  ${CYAN}ignisyeet --help${RESET}"
    box_row "  ${CYAN}cp -r $ex ignisyeet-examples && cd ignisyeet-examples${RESET}"
    box_row "  ${CYAN}ignisyeet sim sample.toml${RESET}"
    box_row "  ${CYAN}ignisyeet-plot all out${RESET}"
  else
    box_row "  ${CYAN}cd \"$(cmdpath "$DEV_DIR")\"${RESET}"
    box_row "  ${CYAN}make all${RESET}                      ${GREY}# sample rocket, end to end${RESET}"
    box_row "  ${CYAN}make -C doc html${RESET}              ${GREY}# documentation${RESET}"
  fi
  box_row ""
  box_bottom
  if [ -n "$CFD_PREFIX" ] && [ "$CFD_ACTION" != none ]; then
    say ""
    say "  ${BOLD}CFD${RESET} ${GREY}- point IgnisYeet at the tools in your .toml:${RESET}"
    say "    ${CYAN}[aero.cfd]${RESET}"
    say "    ${CYAN}prefix = \"$CFD_PREFIX\"${RESET}"
    say "  ${GREY}or: export IGNISYEET_CFD_PREFIX=\"$CFD_PREFIX\"${RESET}"
  fi
  if [ "${PATH_MISSING:-0}" = 1 ]; then
    say ""
    warn "$BIN_DIR is not in your PATH. Add this line to your shell rc file:"
    say "    ${CYAN}export PATH=\"$BIN_DIR:\$PATH\"${RESET}"
  fi
  say ""
  say "  ${GREY}log: $LOG_FILE${RESET}"
  say ""
}

# ----------------------------------------------------------------------------------------------
# Uninstall
# ----------------------------------------------------------------------------------------------
do_uninstall() {
  section "Uninstall"
  local ver dev venv cfd mm
  ver=$(state_get version); dev=$(state_get dev_dir); venv=$(state_get venv); cfd=$(state_get cfd_env); mm=$(state_get micromamba)
  [ -z "$venv" ] && [ -d "$VENV_DIR" ] && venv="$VENV_DIR"
  say "  ${GREY}prefix${RESET}  $PREFIX"
  [ -e "$BIN_DIR/ignisyeet" ] || [ -L "$BIN_DIR/ignisyeet" ] && say "  ${GREY}binary${RESET}  $BIN_DIR/ignisyeet"
  [ -d "$DATA_ROOT" ] && say "  ${GREY}data${RESET}    $DATA_ROOT ${GREY}(${ver:-no state file})${RESET}"
  [ -n "$venv" ] && say "  ${GREY}venv${RESET}    $venv"
  [ -n "$cfd" ] && say "  ${GREY}cfd env${RESET} $cfd"
  [ -n "$dev" ] && say "  ${GREY}clone${RESET}   $dev"
  if [ "$DRY_RUN" = 1 ]; then say ""; say "  ${GREY}dry run: nothing removed${RESET}"; return 0; fi
  if ! ask_yn "Remove the installed binary, links and release data?" y && [ "$HAVE_TTY" = 1 ]; then say "  aborted"; return 0; fi

  local r
  for r in "$BIN_DIR/ignisyeet" "$BIN_DIR/ignisyeet-plot"; do
    if [ -L "$r" ] || [ -f "$r" ]; then rm -f "$r"; ok "removed $r"; fi
  done
  if [ -d "$DATA_ROOT" ]; then
    local d
    for d in "$DATA_ROOT"/v[0-9]* "$DATA_ROOT/current" "$DATA_ROOT/state"; do
      [ -e "$d" ] || [ -L "$d" ] && rm -rf "$d"
    done
    ok "removed release data in $DATA_ROOT"
  fi
  if [ -n "$venv" ] && [ -d "$venv" ]; then
    if [ "$PURGE" = 1 ] || ask_yn "Remove the plotting environment ($venv)?" n; then rm -rf "$venv"; ok "removed $venv"; fi
  fi
  if [ -n "$cfd" ] && [ -d "$cfd" ]; then
    if [ "$PURGE" = 1 ] || ask_yn "Remove the CFD conda environment ($cfd, several GB)?" n; then
      local mgr
      mgr=$(find_conda_manager || true)
      if [ -n "$mgr" ] && "$mgr" env remove -y -p "$cfd" >>"$LOG_FILE" 2>&1 </dev/null; then :; else rm -rf "$cfd"; fi
      ok "removed $cfd"
    fi
  fi
  if [ -n "$mm" ] && [ -x "$mm" ]; then note "micromamba ($mm) was installed by this installer and is kept; remove it manually if unused"; fi
  if [ -n "$dev" ] && [ -d "$dev" ]; then
    if [ "$HAVE_TTY" = 1 ] && ask_yn "Also DELETE the developer clone $dev (any uncommitted work will be lost)?" n; then
      rm -rf "$dev"; ok "removed $dev"
    else
      note "developer clone kept: $dev"
    fi
  fi
  rmdir "$DATA_ROOT" 2>/dev/null || note "kept $LOG_FILE"
  say ""
  ok "uninstall finished"
  say ""
}

# ----------------------------------------------------------------------------------------------
# main
# ----------------------------------------------------------------------------------------------
main() {
  ORIG_ARGS="$*"
  HELP=0
  parse_args "$@"
  setup_ui
  if [ "$HELP" = 1 ]; then usage; exit 0; fi
  trap 'on_err $LINENO' ERR
  trap on_exit EXIT
  trap on_int INT TERM

  detect_tty
  init_paths
  detect_platform
  banner
  [ "$DRY_RUN" = 1 ] && note "dry run: nothing will be changed"
  [ "$HAVE_TTY" = 1 ] || note "non-interactive: using defaults (flags and IGNISYEET_* variables override them)"

  if [ "$ACTION" = uninstall ]; then
    detect_tools
    do_uninstall
    return 0
  fi

  ask_role
  [ -n "$ROLE" ] || ROLE=user
  detect_tools
  show_environment
  decide
  show_plan

  if [ "$DRY_RUN" = 1 ]; then
    say ""
    say "  ${GREY}dry run: nothing was changed.${RESET}"
    say ""
    return 0
  fi
  if [ "$HAVE_TTY" = 1 ]; then
    say ""
    ask_yn "Proceed?" y || { say "  aborted"; exit 0; }
  fi

  CFD_NEED_MM=${CFD_NEED_MM:-0}
  PATH_MISSING=0
  if [ "$ROLE" = user ]; then install_user; else install_developer; fi
  install_cfd
  [ "$ROLE" = user ] && path_hint
  summary
}

main "$@"
