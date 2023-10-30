#!/bin/bash

cargo set-version --workspace $1
pnpm --filter ./wasm/parser/helper build
wasm-pack build wasm/parser --scope wfrs --target nodejs --release
wasm-pack build wasm/runtime --scope wfrs --target web --release
npm version $1 --include-workspace-root -ws --no-git-tag-version --allow-same-version --no-workspaces-update
cp wasm/parser/package_json/package.json wasm/parser/pkg
git add .
git commit -m "build: prepare release v$1"
git push
git tag v$1
git push -u origin v$1
# cargo publish -p wfrs-model
# cargo publish -p wfrs-validator
# cargo publish -p wfrs-engine
# pnpm publish --recursive --access public
