#!/bin/bash

cargo set-version --workspace $1
npm version $1 --include-workspace-root -ws --no-git-tag-version --allow-same-version --no-workspaces-update
cp wasm/parser/package_json/package.json wasm/parser/pkg
git add .
git commit -m "build: prepare release $1"
git push
git tag $1
git push -u origin $1

cargo publish -p wfrs-model
cargo publish -p wfrs-validator
cargo publish -p wfrs-engine
pnpm publish --recursive --access public
