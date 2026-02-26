#!/bin/bash

# Fine-tuning script for xLAM-2 Banking Agent
# Usage: bash scripts/run_training.sh

set -e

echo "Starting xLAM-2 Banking Agent Fine-tuning..."

# Create necessary directories
mkdir -p data/processed
mkdir -p models
mkdir -p logs
mkdir -p results

# Step 1: Prepare dataset
echo "Step 1: Preparing dataset..."
python src/data_preparation.py

# Step 2: Run training
echo "Step 2: Starting training..."
python src/training.py 2>&1 | tee logs/training.log

# Step 3: Run evaluation
echo "Step 3: Evaluating model..."
python src/evaluation.py 2>&1 | tee logs/evaluation.log

echo "Training pipeline completed!"
echo "Check logs/ directory for detailed logs"
echo "Check results/ directory for evaluation results"
echo "Model saved in models/xlam-banking-agent/"