#!/usr/bin/env sh
set -eu

cd website

if ! command -v npm >/dev/null 2>&1; then
    echo "Missing Node.js/npm toolchain" >&2
    exit 1
fi

if [ ! -d node_modules ]; then
    npm ci
fi

npm cache clean --force
npm run lint
npm run build 
npm audit --audit-level=high