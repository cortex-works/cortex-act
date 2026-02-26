# Thai-English One-to-One Distillation Roadmap

This roadmap outlines the systematic process to distill a high-quality Thai-English dataset using the "Agent Distillation" method. This setup ensures continuous, resumable work regardless of token limits or interruptions.

## 1. Architecture

We treat the translation process as a **State-Managed Pipeline**. The Agent acts as the worker, and a local Python script acts as the manager.

- **Source of Truth**: `pure_us_vocab_80k.txt` (The ordered list of words to translate)
- **Database**: `thai_eng_dataset.jsonl` (Append-only storage for results)
- **Manager**: `translation_manager.py` (Determines what to do next)

## 2. Action Plan

### Step 1: Infrastructure Setup (Completed)
We have created `translation_manager.py` to handle the complexity of tracking progress. 
- It scans the current `thai_eng_dataset.jsonl`.
- It compares it against `pure_us_vocab_80k.txt`.
- It serves the exact next batch of words needed.

### Step 2: The Translation Loop (Standard Operating Procedure)

For you (the prompt engineer) to run the agent, simply follow this loop. The Agent will execute these steps autonomously when asked to "Translate the next batch".

**The Agent's Workflow:**
1.  **Check State**: Run `python translation_manager.py --status` to see progress.
2.  **Fetch Work**: Run `python translation_manager.py --next [N]` (Recommended N=50 or 100).
3.  **Distill (Translate)**:
    - The Agent reads the list of English words.
    - Uses internal knowledge to generate `{"en": "word", "th": "คำแปล"}` pairs.
    - Ensures 1:1 mapping with the most common Thai meaning.
4.  **Commit**:
    - Saves the generated lines to a temporary file `temp_batch.jsonl`.
    - Appends to the main dataset: `cat temp_batch.jsonl >> thai_eng_dataset.jsonl`.
    - Deletes the temp file.

### Step 3: Dataset Specifications

- **Format**: JSONL (JSON Lines)
- **Schema**: `{"en": "string", "th": "string"}`
- **Constraint**: One word, one primary meaning. No explanations. Simple vocabulary mapping.

## 3. How to Start

You (the user) just need to prompt the Agent with:

> **"Start the translation loop. Translate the next batch of 50 words."**

The Agent will handle the rest. When it stops, you simply say **"Continue"**, and it will pick up exactly where it left off because `translation_manager.py` tracks the state on disk.
