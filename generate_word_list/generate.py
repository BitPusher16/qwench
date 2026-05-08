#!/usr/bin/env python3
"""
Downloads Google Books Ngram v2 1-gram data (English, 20120701),
computes the 200 most common words per letter of the alphabet,
writes them to a text file, then generates a Rust source file
with a hardcoded function returning all words as a Vec<&'static str>.

Usage:
    python ngram_to_rust.py

Outputs:
    top_words.txt   — plain text, one word per line, grouped by letter
    word_list.rs   — Rust source file with hardcoded word lists
"""

import gzip
import os
import re
import shutil
import string
import urllib.request
from collections import defaultdict

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

# Google Books Ngrams v2, English, 1-grams (confirmed working URL pattern).
# One gzipped file per letter of the alphabet.
NGRAM_URL_PATTERN = (
    "http://storage.googleapis.com/books/ngrams/books/"
    "googlebooks-eng-all-1gram-20120701-{letter}.gz"
)

CACHE_DIR = "ngram_cache"   # downloaded .gz files are kept here
TOP_N = 200
OUTPUT_TXT = "top_words.txt"
OUTPUT_RS = "word_list.rs"

ALPHABET = list(string.ascii_lowercase)

# Accept only purely alphabetic words (no digits, hyphens, POS tags, etc.)
WORD_RE = re.compile(r'^[a-z]+$')


# ---------------------------------------------------------------------------
# Download helpers
# ---------------------------------------------------------------------------

def get_cached_path(letter: str) -> str:
    """Return the local cache path for a letter's .gz file."""
    os.makedirs(CACHE_DIR, exist_ok=True)
    filename = f"googlebooks-eng-all-1gram-20120701-{letter}.gz"
    return os.path.join(CACHE_DIR, filename)


def ensure_downloaded(letter: str) -> str:
    """Download the .gz file for *letter* if not already cached; return its path."""
    path = get_cached_path(letter)
    if os.path.exists(path):
        print(f"  Cache hit:  {path}")
        return path

    url = NGRAM_URL_PATTERN.format(letter=letter)
    print(f"  Downloading {url} ...", end=" ", flush=True)
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=300) as resp:
        with open(path, "wb") as f:
            shutil.copyfileobj(resp, f)
    print(f"saved to {path}")
    return path


def download_letter(letter: str) -> dict:
    """Return {word: total_count} for *letter*, using the disk cache."""
    path = ensure_downloaded(letter)

    counts: dict = defaultdict(int)
    with gzip.open(path, "rt", encoding="utf-8", errors="replace") as f:
        for line in f:
            # Format: ngram TAB year TAB match_count TAB volume_count
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 3:
                continue

            word_raw = parts[0]
            match_count_str = parts[2]

            # Strip part-of-speech suffixes like "_NOUN", "_VERB", etc.
            word = word_raw.split("_")[0].lower()

            if not WORD_RE.match(word):
                continue
            if not (6 <= len(word) <= 10):
                continue
            if not word.startswith(letter):
                continue  # skip any mis-filed entries

            try:
                counts[word] += int(match_count_str)
            except ValueError:
                continue

    print(f"  Parsed {len(counts):,} unique words")
    return counts


# ---------------------------------------------------------------------------
# Main pipeline
# ---------------------------------------------------------------------------

def compute_top_words() -> dict:
    top = {}
    for letter in ALPHABET:
        print(f"[{letter.upper()}]")
        counts = download_letter(letter)
        sorted_words = sorted(counts, key=lambda w: counts[w], reverse=True)
        top[letter] = sorted_words[:TOP_N]
        if top[letter]:
            best = top[letter][0]
            print(f"  Top word: '{best}'  ({counts[best]:,} occurrences)")
    return top


def write_text_file(top: dict, path: str) -> None:
    with open(path, "w", encoding="utf-8") as f:
        for letter in ALPHABET:
            f.write(f"# {letter.upper()}\n")
            for word in top[letter]:
                f.write(word + "\n")
            f.write("\n")
    total = sum(len(top[l]) for l in ALPHABET)
    print(f"\nWrote {total} words to '{path}'")


def read_text_file(path: str) -> dict:
    top = defaultdict(list)
    current_letter = None
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if line.startswith("# ") and len(line) == 3:
                current_letter = line[2].lower()
            elif line and current_letter is not None:
                top[current_letter].append(line)
    return top


def write_rust_file(top: dict, path: str) -> None:
    all_words = [word for letter in ALPHABET for word in top.get(letter, [])]
 
    out = []
    out.append("// Auto-generated by ngram_to_rust.py")
    out.append("// Source: Google Books Ngrams v2 (English 1-grams, 20120701)")
    out.append("// Top 200 most common words per letter of the English alphabet.")
    out.append("")
    out.append(f"/// All {len(all_words)} most common English words (top 200 per letter, a-z).")
    out.append("pub fn all_words() -> Vec<String> {")
    out.append("    vec![")
    for word in all_words:
        escaped = word.replace("\\", "\\\\").replace('"', '\\"'  )
        out.append(f'        "{escaped}".to_string(),')
    out.append("    ]")
    out.append("}")
    out.append("")
 
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(out))
 
    print(f"Wrote Rust source to '{path}'  ({len(all_words)} hardcoded words in a single function)")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    print("=== Step 1: Download ngram data and compute top words ===\n")
    top_words = compute_top_words()

    print("\n=== Step 2: Write word lists to text file ===")
    write_text_file(top_words, OUTPUT_TXT)

    print("\n=== Step 3: Read text file back and generate Rust source ===")
    top_from_file = read_text_file(OUTPUT_TXT)
    write_rust_file(top_from_file, OUTPUT_RS)

    print("\nAll done!")
    print(f"  {OUTPUT_TXT}  -- plain-text word lists (one word per line)")
    print(f"  {OUTPUT_RS}    -- Rust source with 26 hardcoded Vec<&'static str> functions")
