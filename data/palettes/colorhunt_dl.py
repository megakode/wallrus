#!/usr/bin/env python3
"""
ColorHunt Palette Downloader

Crawls colorhunt.co and generates 1x4 palette images locally from hex codes to not hammer their download.
Each image is 1 pixel wide and 4 pixels tall — one pixel per color, top to bottom.

Usage:
    python colorhunt_dl.py pastel                    # download 100 pastel palettes
    python colorhunt_dl.py dark --limit 500          # download 500 dark palettes
    python colorhunt_dl.py neon --sort popular        # download popular neon palettes
    python colorhunt_dl.py sunset -o ./my_dir         # custom output directory
    python colorhunt_dl.py --list-categories          # show available categories

Images are saved to ./<category>/ by default.

Categories:
    pastel, vintage, retro, neon, gold, light, dark, warm, cold, summer, fall,
    winter, spring, happy, nature, earth, night, space, rainbow, gradient, sunset,
    sky, sea, kids, skin, food, cream, coffee, wedding, christmas, halloween
"""

import argparse
import os
import sys
import time
from pathlib import Path

try:
    import requests
except ImportError:
    print("Error: 'requests' package is required. Install with: pip install requests")
    sys.exit(1)

try:
    from PIL import Image
except ImportError:
    print("Error: 'Pillow' package is required. Install with: pip install Pillow")
    sys.exit(1)


FEED_URL = "https://colorhunt.co/php/feed.php"

CATEGORIES = [
    "pastel", "vintage", "retro", "neon", "gold", "light", "dark", "warm",
    "cold", "summer", "fall", "winter", "spring", "happy", "nature", "earth",
    "night", "space", "rainbow", "gradient", "sunset", "sky", "sea", "kids",
    "skin", "food", "cream", "coffee", "wedding", "christmas", "halloween",
]

SORT_OPTIONS = ["new", "popular", "random"]

HEADERS = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
    "Referer": "https://colorhunt.co/",
    "Origin": "https://colorhunt.co",
    "X-Requested-With": "XMLHttpRequest",
    "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
}


def parse_palette_code(code: str) -> list[str]:
    """Parse a 24-char hex code into 4 hex color strings."""
    if len(code) != 24:
        raise ValueError(f"Invalid palette code length: {len(code)} (expected 24)")
    return [f"#{code[i*6:(i+1)*6].upper()}" for i in range(4)]


def generate_palette_image(colors: list[str], filepath: Path) -> None:
    """Generate a 1x4 PNG with one pixel per color (top to bottom)."""
    img = Image.new("RGB", (1, 4))
    for i, color in enumerate(colors):
        r = int(color[1:3], 16)
        g = int(color[3:5], 16)
        b = int(color[5:7], 16)
        img.putpixel((0, i), (r, g, b))
    img.save(filepath, "PNG")


def fetch_palettes(step: int, sort: str, tags: str, timeframe: int = 30) -> list[dict]:
    """Fetch a page of palettes from the ColorHunt feed API."""
    data = {
        "step": step,
        "sort": sort,
        "tags": tags,
        "timeframe": timeframe,
    }
    resp = requests.post(FEED_URL, data=data, headers=HEADERS, timeout=15)
    resp.raise_for_status()
    text = resp.text.strip()
    if not text or text == "[]":
        return []
    return resp.json()


def crawl_and_download(
    output_dir: Path,
    limit: int,
    category: str,
    sort: str,
    delay: float,
    timeframe: int,
) -> None:
    """Main crawl loop: fetch palette codes from API, generate images locally."""
    output_dir.mkdir(parents=True, exist_ok=True)

    collected_codes: list[str] = []
    step = 0
    empty_pages = 0
    max_empty = 3  # stop after 3 consecutive empty responses

    print(f"Output directory: {output_dir.resolve()}")
    print(f"Target: {limit} palettes | Sort: {sort} | Category: {category or 'all'}")
    print(f"Delay between API requests: {delay}s")
    print()

    # Phase 1: Collect palette codes from the API
    while len(collected_codes) < limit:
        try:
            palettes = fetch_palettes(step, sort, category, timeframe)
        except requests.RequestException as e:
            print(f"  [!] API request failed (step={step}): {e}")
            empty_pages += 1
            if empty_pages >= max_empty:
                print("  [!] Too many consecutive failures, stopping.")
                break
            time.sleep(delay * 2)
            step += 1
            continue

        if not palettes:
            empty_pages += 1
            if empty_pages >= max_empty:
                print(f"  Reached end of results after {step} pages.")
                break
            step += 1
            time.sleep(delay)
            continue

        empty_pages = 0  # reset on success

        for item in palettes:
            code = item.get("code", "")
            if len(code) == 24 and code not in collected_codes:
                collected_codes.append(code)
                if len(collected_codes) >= limit:
                    break

        print(f"  Fetched page {step}: got {len(palettes)} palettes "
              f"({len(collected_codes)}/{limit} collected)")

        step += 1
        time.sleep(delay)

    if not collected_codes:
        print("No palettes found. Check your category/sort options.")
        return

    print(f"\nCollected {len(collected_codes)} palette codes. Generating images...\n")

    # Phase 2: Generate images locally
    created = 0
    skipped = 0
    errors = 0

    for i, code in enumerate(collected_codes, 1):
        filepath = output_dir / f"{code}.png"

        if filepath.exists():
            skipped += 1
            continue

        try:
            colors = parse_palette_code(code)
            generate_palette_image(colors, filepath)
            created += 1
        except Exception as e:
            print(f"  [!] Error generating {code}: {e}")
            errors += 1
            continue

        # Progress every 25 images or on the last one
        if i % 25 == 0 or i == len(collected_codes):
            print(f"  Progress: {i}/{len(collected_codes)} "
                  f"(created: {created}, skipped: {skipped}, errors: {errors})")

    print(f"\nDone! Created: {created} | Skipped (existing): {skipped} | Errors: {errors}")
    print(f"Images saved to: {output_dir.resolve()}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Download color palette images from colorhunt.co",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--limit", "-n",
        type=int,
        default=100,
        help="Maximum number of palettes to download (default: 100)",
    )
    parser.add_argument(
        "category",
        type=str,
        help=f"Category tag to download (e.g. pastel, dark, neon). "
             f"Use --list-categories to see all options.",
        nargs="?",
        default=None,
    )
    parser.add_argument(
        "--sort", "-s",
        type=str,
        default="new",
        choices=SORT_OPTIONS,
        help="Sort order: new, popular, random (default: new)",
    )
    parser.add_argument(
        "--output", "-o",
        type=str,
        default=None,
        help="Output directory for images (default: ./<category>)",
    )
    parser.add_argument(
        "--delay", "-d",
        type=float,
        default=0.5,
        help="Delay in seconds between API requests (default: 0.5)",
    )
    parser.add_argument(
        "--timeframe",
        type=int,
        default=4000,
        choices=[30, 365, 4000],
        help="Timeframe for 'popular' sort: 30 (month), 365 (year), 4000 (all time). Default: 4000",
    )
    parser.add_argument(
        "--list-categories",
        action="store_true",
        help="List all available categories and exit",
    )

    args = parser.parse_args()

    if args.list_categories:
        print("Available categories:")
        for cat in CATEGORIES:
            print(f"  {cat}")
        return

    if not args.category:
        parser.error("category is required (e.g. 'pastel', 'dark', 'neon'). "
                     "Use --list-categories to see all options.")

    category = args.category.lower()

    if category not in CATEGORIES:
        print(f"Warning: '{category}' is not a known category. Proceeding anyway...")

    output_dir = Path(args.output) if args.output else Path(f"./{category}")

    crawl_and_download(
        output_dir=output_dir,
        limit=args.limit,
        category=category,
        sort=args.sort,
        delay=args.delay,
        timeframe=args.timeframe,
    )


if __name__ == "__main__":
    main()
