#!/usr/bin/env python3
"""
Pure US English Vocabulary Generator
Uses Hybrid Intersection Methodology to filter high-frequency words against SCOWL dictionary
Author: Senior NLP Engineer
Date: 2026-01-16
"""

import os
import re
import zipfile
import requests
from pathlib import Path
from typing import Set
import wordfreq


# Configuration
SCOWL_URL = "https://sourceforge.net/projects/wordlist/files/SCOWL/2020.12.07/scowl-2020.12.07.zip/download"
SCOWL_DIR = "scowl_data"
SCOWL_ARCHIVE = "scowl-2020.12.07.zip"
OUTPUT_FILE = "pure_us_vocab_80k.txt"

# Sizes to include (exclude 80+ to avoid obscure words)
VALID_SIZES = [10, 20, 35, 40, 50, 60, 70]

# Word list patterns to include (US English only)
INCLUDE_PATTERNS = [
    "english-words",
    "american-words"
]

# Patterns to exclude (non-US variants)
EXCLUDE_PATTERNS = [
    "british-words",
    "canadian-words",
    "british_",
    "canadian_"
]


def download_scowl():
    """Download SCOWL database if not present."""
    if os.path.exists(SCOWL_DIR) and os.listdir(SCOWL_DIR):
        print(f"✓ SCOWL data already exists in {SCOWL_DIR}/")
        return
    
    print(f"Downloading SCOWL database from {SCOWL_URL}...")
    print("This may take a moment...")
    
    try:
        response = requests.get(SCOWL_URL, allow_redirects=True, timeout=60)
        response.raise_for_status()
        
        with open(SCOWL_ARCHIVE, 'wb') as f:
            f.write(response.content)
        
        print(f"✓ Downloaded {SCOWL_ARCHIVE}")
        
        # Extract the archive
        print("Extracting SCOWL archive...")
        with zipfile.ZipFile(SCOWL_ARCHIVE, 'r') as zip_ref:
            zip_ref.extractall(SCOWL_DIR)
        
        print(f"✓ Extracted to {SCOWL_DIR}/")
        
        # Clean up archive
        os.remove(SCOWL_ARCHIVE)
        print("✓ Cleaned up archive file")
        
    except Exception as e:
        print(f"✗ Error downloading SCOWL: {e}")
        print("\nAlternative: Download manually from:")
        print("https://sourceforge.net/projects/wordlist/files/SCOWL/")
        print(f"Extract to: {SCOWL_DIR}/")
        raise


def load_us_words() -> Set[str]:
    """
    Load valid US English words from SCOWL database.
    Only loads english-words.* and american-words.* files
    with sizes 10, 20, 35, 40, 50, 60, 70.
    """
    valid_words = set()
    
    # Find the SCOWL final directory
    scowl_path = Path(SCOWL_DIR)
    final_dirs = list(scowl_path.glob("**/final"))
    
    if not final_dirs:
        raise FileNotFoundError(f"Could not find 'final' directory in {SCOWL_DIR}/")
    
    final_dir = final_dirs[0]
    print(f"\n✓ Found SCOWL final directory: {final_dir}")
    
    loaded_files = []
    
    # Iterate through all files in the final directory
    for filepath in final_dir.iterdir():
        if not filepath.is_file():
            continue
        
        filename = filepath.name
        
        # Check if file matches our include patterns
        matches_include = any(pattern in filename for pattern in INCLUDE_PATTERNS)
        if not matches_include:
            continue
        
        # Exclude British/Canadian variants
        matches_exclude = any(pattern in filename for pattern in EXCLUDE_PATTERNS)
        if matches_exclude:
            continue
        
        # Extract size from filename (format: word-list.SIZE or similar)
        size_match = re.search(r'\.(\d+)(?:\.|$)', filename)
        if not size_match:
            continue
        
        size = int(size_match.group(1))
        
        # Only include specified sizes
        if size not in VALID_SIZES:
            continue
        
        # Load words from file
        try:
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                words = {line.strip().lower() for line in f if line.strip()}
                valid_words.update(words)
                loaded_files.append(f"{filename} ({len(words)} words)")
        except Exception as e:
            print(f"Warning: Could not load {filename}: {e}")
    
    print(f"\n✓ Loaded {len(loaded_files)} SCOWL files:")
    for file_info in sorted(loaded_files):
        print(f"  - {file_info}")
    
    print(f"\n✓ Total unique US English words in dictionary: {len(valid_words):,}")
    
    return valid_words


def is_valid_word(word: str) -> bool:
    """
    Check if word meets validity criteria:
    - Only contains lowercase letters [a-z]
    - Length >= 2, OR is 'a' or 'i'
    """
    # Must be only lowercase letters
    if not re.match(r'^[a-z]+$', word):
        return False
    
    # Allow 'a' and 'i' as single-letter words, require length >= 2 for others
    if len(word) < 2 and word not in {'a', 'i'}:
        return False
    
    return True


def generate_pure_us_vocab():
    """Main function to generate pure US English vocabulary."""
    print("=" * 70)
    print("PURE US ENGLISH VOCABULARY GENERATOR")
    print("Hybrid Intersection Methodology")
    print("=" * 70)
    
    # Step 1: Download SCOWL if needed
    print("\n[STEP 1] Checking SCOWL Database...")
    download_scowl()
    
    # Step 2: Load valid US words from SCOWL
    print("\n[STEP 2] Loading US English Dictionary (SCOWL)...")
    valid_us_words = load_us_words()
    
    if not valid_us_words:
        raise ValueError("No valid words loaded from SCOWL database!")
    
    # Step 3: Get high-frequency word candidates
    print("\n[STEP 3] Extracting Top 100,000 Frequent Words (wordfreq)...")
    candidates = wordfreq.top_n_list('en', 100000)
    print(f"✓ Extracted {len(candidates):,} candidate words")
    
    # Step 4: Apply hybrid intersection filtering
    print("\n[STEP 4] Applying Hybrid Intersection Filter...")
    pure_vocab = []
    
    stats = {
        'total_candidates': len(candidates),
        'in_dictionary': 0,
        'rejected_invalid_chars': 0,
        'rejected_too_short': 0,
        'accepted': 0
    }
    
    for word in candidates:
        word_lower = word.lower()
        
        # Check if in valid US dictionary
        if word_lower not in valid_us_words:
            continue
        
        stats['in_dictionary'] += 1
        
        # Apply validity filters
        if not is_valid_word(word_lower):
            if not re.match(r'^[a-z]+$', word_lower):
                stats['rejected_invalid_chars'] += 1
            else:
                stats['rejected_too_short'] += 1
            continue
        
        # Word passed all filters
        pure_vocab.append(word_lower)
        stats['accepted'] += 1
    
    # Step 5: Save output
    print("\n[STEP 5] Saving Output...")
    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        for word in pure_vocab:
            f.write(f"{word}\n")
    
    print(f"✓ Saved to: {OUTPUT_FILE}")
    
    # Print statistics
    print("\n" + "=" * 70)
    print("GENERATION COMPLETE")
    print("=" * 70)
    print(f"\nStatistics:")
    print(f"  Total candidates:            {stats['total_candidates']:>8,}")
    print(f"  Found in US dictionary:      {stats['in_dictionary']:>8,}")
    print(f"  Rejected (invalid chars):    {stats['rejected_invalid_chars']:>8,}")
    print(f"  Rejected (too short):        {stats['rejected_too_short']:>8,}")
    print(f"  ─────────────────────────────────────")
    print(f"  ✓ FINAL PURE US VOCABULARY:  {stats['accepted']:>8,}")
    print(f"\nOutput file: {OUTPUT_FILE}")
    print(f"Target range: 50,000 - 80,000 words")
    
    if 50000 <= stats['accepted'] <= 80000:
        print(f"✓ SUCCESS: Within target range!")
    elif stats['accepted'] < 50000:
        print(f"⚠ Note: Below target range (consider including size 80)")
    else:
        print(f"⚠ Note: Above target range (consider reducing sizes)")
    
    print("=" * 70)


if __name__ == "__main__":
    try:
        generate_pure_us_vocab()
    except KeyboardInterrupt:
        print("\n\nOperation cancelled by user.")
    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback
        traceback.print_exc()
