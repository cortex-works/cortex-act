#!/usr/bin/env python3
"""
Translation Manager for Distillation Pipeline
Manage state between pure_us_vocab and thai_eng_dataset
"""

import os
import json
import argparse
import sys

SOURCE_FILE = "pure_us_vocab_80k.txt"
DATASET_FILE = "thai_eng_dataset.jsonl"

def load_vocab():
    if not os.path.exists(SOURCE_FILE):
        print(f"Error: {SOURCE_FILE} not found.")
        sys.exit(1)
    with open(SOURCE_FILE, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f if line.strip()]

def load_progress():
    processed_words = set()
    if os.path.exists(DATASET_FILE):
        with open(DATASET_FILE, 'r', encoding='utf-8') as f:
            for line in f:
                try:
                    data = json.loads(line)
                    if 'en' in data:
                        processed_words.add(data['en'])
                except json.JSONDecodeError:
                    continue
    return processed_words

def get_status():
    all_words = load_vocab()
    processed = load_progress()
    total = len(all_words)
    done = len(processed)
    remaining = total - done
    
    # Calculate restart index
    # Note: We stick to sequential order from the source file for consistency
    next_index = 0
    for i, word in enumerate(all_words):
        if word not in processed:
            next_index = i
            break
            
    # Check if completely done
    if done == total:
        next_index = total

    return {
        "total": total,
        "done": done,
        "remaining": remaining,
        "percent": (done / total * 100) if total > 0 else 0,
        "next_index": next_index,
        "all_words": all_words
    }

def main():
    parser = argparse.ArgumentParser(description="Manage Translation Pipeline")
    parser.add_argument("--status", action="store_true", help="Show current progress")
    parser.add_argument("--next", type=int, default=0, help="Get next N words to translate")
    args = parser.parse_args()

    state = get_status()

    if args.status:
        print(f"Status Report:")
        print(f"  Total Vocab:    {state['total']:,}")
        print(f"  Translated:     {state['done']:,}")
        print(f"  Remaining:      {state['remaining']:,}")
        print(f"  Progress:       {state['percent']:.2f}%")
        print(f"  Next Word idx:  {state['next_index']}")
    
    if args.next > 0:
        if state['remaining'] == 0:
            print("Done! No more words to translate.")
            return

        start = state['next_index']
        end = min(start + args.next, len(state['all_words']))
        batch = state['all_words'][start:end]
        
        # Output as simple list for Agent to consume
        print(f"\n--- BATCH START ({len(batch)} words) ---")
        for word in batch:
            print(word)
        print("--- BATCH END ---")

if __name__ == "__main__":
    main()
