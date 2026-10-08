#!/usr/bin/env bash
# Checks the error/warning codes of install.sh: every die/warn call carries a code, every code is documented
# in doc/source/errors/install.rst, and the rendered messages look like error[IY-Innn]: ... / warning[IY-Innn]: ...
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
fail=0
bad() { echo "FAIL: $*" >&2; fail=1; }

if grep -nE '^[^#]*\b(die|warn) +"' "$root/install.sh"; then bad "die/warn call without a code"; fi
codes=$(grep -ohE '(die|warn) +IY-I[0-9]{3}' "$root/install.sh" | grep -oE 'IY-I[0-9]{3}' | sort -u)
codes=$(printf '%s\n' "$codes" IY-I090 IY-I501 IY-I060 IY-I061 | sort -u)
for c in $codes; do
  grep -q "^\.\. _$c:" "$root/doc/source/errors/install.rst" || bad "$c is not documented"
done
while read -r c; do
  grep -rq "$c" "$root/install.sh" "$root/scripts/package-release.sh" || bad "$c is documented but unused"
done < <(grep -oE '^\.\. _IY-I[0-9]{3}:' "$root/doc/source/errors/install.rst" | grep -oE 'IY-I[0-9]{3}')

out=$(bash "$root/install.sh" --no-color --bogus 2>&1); rc=$?
[ "$rc" = 1 ] || bad "exit status $rc for an unknown option (expected 1)"
printf '%s\n' "$out" | grep -qE 'error\[IY-I001\]: unknown option: --bogus' || bad "unknown option message: $out"
out=$(bash "$root/install.sh" --no-color --version 1.2 2>&1)
printf '%s\n' "$out" | grep -qE 'error\[IY-I003\]: version must match' || bad "version message: $out"
[ "$fail" = 0 ] && echo "install error codes: ok"
exit "$fail"
