# Agent Fine-Tuning Workshop Structure

## Workshop Overview
This workshop demonstrates fine-tuning the Salesforce xLAM-2-1b-fc-r model using the Galileo AI agent leaderboard v2 dataset for improved banking agent performance.

## Dataset Details
- **Source**: galileo-ai/agent-leaderboard-v2
- **Domain**: Banking (100 scenarios)
- **Features**: Multi-turn conversations, complex tool usage, realistic customer support scenarios
- **Evaluation Metrics**: Action Completion (AC) and Tool Selection Quality (TSQ)

## Base Model Details
- **Model**: Salesforce/xLAM-2-1b-fc-r
- **Size**: 1B parameters
- **Context**: 32k tokens (max 128k with YaRN)
- **Specialization**: Multi-turn conversation and function calling

## Workshop Components
1. Environment Setup
2. Dataset Preparation and Analysis
3. Model Configuration
4. Training Pipeline
5. Evaluation Framework
6. Deployment and Testing