#!/bin/bash

set -e

(
    cd ./macro-support
    cargo publish --no-verify
)

cargo publish --no-verify