#!/usr/bin/env bash
# Optimize images in the frontend static assets directory.
# Converts PNG/JPG to WebP, generates responsive srcset variants.
# Requires: cwebp (from libwebp), convert (from imagemagick)
set -euo pipefail

ASSETS_DIR="${1:-frontend/static/assets}"
WIDTHS=(320 640 1024)

if ! command -v cwebp &>/dev/null; then
  echo "cwebp not found. Install libwebp (pacman -S libwebp)" >&2
  exit 1
fi

count=0
for img in "$ASSETS_DIR"/*.{png,jpg,jpeg} 2>/dev/null; do
  [[ -f "$img" ]] || continue
  base="${img%.*}"
  ext="${img##*.}"

  # Convert to WebP if not already done
  webp="${base}.webp"
  if [[ ! -f "$webp" ]]; then
    cwebp -q 80 "$img" -o "$webp" 2>/dev/null
    echo "  created $webp"
    ((count++))
  fi

  # Generate responsive variants
  if command -v convert &>/dev/null; then
    for w in "${WIDTHS[@]}"; do
      variant="${base}-${w}w.webp"
      if [[ ! -f "$variant" ]]; then
        convert "$img" -resize "${w}>" -quality 80 "$variant" 2>/dev/null
        echo "  created $variant"
        ((count++))
      fi
    done
  fi
done

echo "optimized $count images"
