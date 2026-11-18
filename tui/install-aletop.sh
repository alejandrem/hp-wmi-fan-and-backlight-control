#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: instala Rust/Cargo antes de instalar Ale Top." >&2
    exit 1
fi

cargo install --path "$project_dir" --bin aletop --force
echo "Ale Top quedó instalado. Ejecuta: aletop"
