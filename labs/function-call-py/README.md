# Agent Fine-Tuning Workshop: xLAM-2 Banking Agent

## Overview
This comprehensive workshop demonstrates fine-tuning the **Salesforce xLAM-2-1b-fc-r** model using the **Galileo AI agent leaderboard v2** dataset to create a specialized banking customer support agent.

### Key Features
- **Dataset**: 100 banking scenarios with multi-turn conversations
- **Base Model**: xLAM-2-1b-fc-r (1B parameters, optimized for function calling)
- **Evaluation**: Action Completion (AC) and Tool Selection Quality (TSQ) metrics
- **Training**: LoRA fine-tuning for efficient adaptation
- **Deployment**: Interactive demo and API-compatible inference

## Dataset Details
- **Source**: `galileo-ai/agent-leaderboard-v2` (banking domain)
- **Scenarios**: 100 complex customer support interactions
- **Goals per scenario**: 6-8 banking tasks (transfers, disputes, card management, etc.)
- **Personas**: Diverse customer profiles with varying communication styles
- **Tools**: Banking-specific function definitions for realistic agent behavior

## Base Model Specifications
- **Model**: Salesforce/xLAM-2-1b-fc-r
- **Parameters**: 1B (efficient for deployment)
- **Context Length**: 32k tokens (extendable to 128k)
- **Specialization**: Multi-turn conversations and function calling
- **Architecture**: Qwen-2.5 based with function calling optimizations

## Prerequisites
- Python 3.8+
- CUDA-compatible GPU (8GB+ VRAM recommended)
- Hugging Face account with API token
- 16GB+ system RAM
- Basic understanding of transformers and fine-tuning

## Quick Start
```bash
# Clone and setup
git clone <repository>
cd agent-fine-tuning-workshop
pip install -r requirements.txt

# Run complete pipeline
bash scripts/run_training.sh

# Or run interactive demo
python scripts/inference_demo.py
```

## Workshop Structure
1. **Environment Setup** - Dependencies and configuration
2. **Dataset Exploration** - Understanding the banking scenarios
3. **Data Preparation** - Processing for xLAM-2 format
4. **Model Training** - LoRA fine-tuning pipeline
5. **Evaluation Framework** - AC and TSQ metrics
6. **Interactive Demo** - Test the trained agent
7. **Deployment Guide** - Production considerations

## Getting Started
Choose your preferred approach:

### 📓 Jupyter Notebooks (Recommended for Learning)
1. `notebooks/01_dataset_exploration.ipynb` - Explore the banking dataset
2. `notebooks/02_model_training.ipynb` - Fine-tuning walkthrough
3. `notebooks/03_evaluation_analysis.ipynb` - Performance analysis

### 🚀 Command Line (Recommended for Training)
```bash
# Complete pipeline
bash scripts/run_training.sh

# Individual steps
python src/data_preparation.py
python src/training.py
python src/evaluation.py
```

### 🎮 Interactive Demo
```bash
python scripts/inference_demo.py
```

## Expected Results
- **Training Time**: 2-4 hours on single GPU
- **AC Score**: 0.75+ (vs 0.65 baseline)
- **TSQ Score**: 0.80+ (vs 0.70 baseline)
- **Model Size**: ~2GB (with LoRA adapters)

## File Structure
```
agent-fine-tuning-workshop/
├── notebooks/           # Jupyter notebooks for interactive learning
├── src/                # Core training and evaluation code
├── scripts/            # Automation scripts and demo
├── config/             # Training configurations
├── data/               # Dataset storage (auto-created)
├── models/             # Trained models (auto-created)
├── results/            # Evaluation results (auto-created)
└── requirements.txt    # Python dependencies
```

## Workshop Highlights

### 🎯 What You'll Learn
- How to process enterprise-grade agent datasets
- Fine-tuning techniques for function-calling models
- Evaluation metrics for agent performance (AC & TSQ)
- Production deployment considerations
- Interactive agent testing and validation

### 🔧 Technical Stack
- **Framework**: Transformers, PEFT (LoRA), TRL
- **Model**: xLAM-2-1b-fc-r (Salesforce)
- **Dataset**: Galileo AI Agent Leaderboard v2
- **Training**: LoRA fine-tuning with gradient accumulation
- **Evaluation**: Custom AC/TSQ metrics
- **Monitoring**: Weights & Biases integration

### 📊 Expected Performance
Based on the xLAM-2 paper and similar fine-tuning experiments:
- **Baseline xLAM-2-1b**: AC ~0.65, TSQ ~0.70
- **Fine-tuned Banking Agent**: AC ~0.75+, TSQ ~0.80+
- **Training Efficiency**: 2-4 hours on single GPU
- **Memory Usage**: ~8GB VRAM with LoRA

## Advanced Features

### Multi-Turn Conversation Support
The workshop includes handling of complex multi-turn banking conversations with context preservation and tool coordination.

### Banking-Specific Tools
Realistic banking functions including:
- Account transfers and balance checks
- Credit card management and disputes
- Loan applications and payments
- Currency exchange and international transfers
- Branch/ATM location services

### Evaluation Framework
Comprehensive evaluation using:
- **Action Completion (AC)**: Measures goal achievement
- **Tool Selection Quality (TSQ)**: Evaluates tool usage accuracy
- **Persona-based Analysis**: Performance across customer types
- **Complexity Analysis**: Performance vs. scenario difficulty

## Troubleshooting

### Common Issues
1. **CUDA Out of Memory**: Reduce batch size or use gradient checkpointing
2. **Slow Training**: Enable mixed precision (bf16) and optimize data loading
3. **Poor Performance**: Check data preprocessing and increase training epochs
4. **Evaluation Errors**: Ensure test data format matches training data

### Performance Optimization
- Use LoRA for memory efficiency
- Enable gradient accumulation for effective larger batch sizes
- Use mixed precision training (bf16)
- Optimize data loading with multiple workers

## Contributing
This workshop is designed for educational purposes. Feel free to:
- Extend to other domains (healthcare, insurance, etc.)
- Experiment with different base models
- Improve evaluation metrics
- Add more sophisticated tool definitions

## License
This workshop is provided for educational and research purposes. Please respect the licenses of the underlying models and datasets:
- xLAM-2 models: Salesforce Research License
- Galileo AI dataset: Check dataset license
- Code: MIT License (workshop components)

## Citation
If you use this workshop in your research, please cite:
```bibtex
@misc{xlam-banking-workshop,
  title={Agent Fine-Tuning Workshop: xLAM-2 Banking Agent},
  year={2025},
  howpublished={\url{https://github.com/your-repo/agent-fine-tuning-workshop}}
}
```

## Support
For questions and issues:
1. Check the troubleshooting section
2. Review the notebook examples
3. Open an issue on GitHub
4. Consult the xLAM-2 and Galileo AI documentation