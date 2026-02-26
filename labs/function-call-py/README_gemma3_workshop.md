# 🚀 Dynamic Multi-Function Calling Workshop: Gemma 3 + Ollama

## Overview
This comprehensive workshop demonstrates how to build a **dynamic multi-function calling system using Gemma 3 running locally with Ollama**. You'll learn to create a privacy-preserving AI assistant that can interact with external tools through structured function calls.

### Key Features
- **Local Execution**: Everything runs locally ensuring data privacy
- **Multi-Function Support**: Real-time search, translation, weather, and more
- **Dynamic Tool Selection**: AI decides which tools to use based on context
- **Structured Outputs**: JSON-based function calling without direct API execution
- **Extensible Architecture**: Easy to add new functions and capabilities

## What You'll Build
A local AI assistant that can:
- 🔍 **Search the web** using Serper.dev API
- 🌐 **Translate text** using MyMemory API
- ⛅ **Fetch weather data** using OpenWeatherMap API
- 🧠 **Answer from memory** when information is already known
- 🔄 **Dynamically choose** the right tool for each query

## Base Model Details
- **Model**: Google Gemma 3 (2B/9B/27B variants)
- **Runtime**: Ollama (local inference)
- **Context**: Up to 8K tokens
- **Specialization**: Instruction following and structured output generation

## Prerequisites
- Python 3.8+
- 8GB+ RAM (16GB recommended for larger models)
- Ollama installed
- API keys for external services (optional)
- Basic understanding of JSON and API concepts

## Quick Start
```bash
# Clone and setup
git clone <repository>
cd gemma3-function-calling-workshop
pip install -r requirements.txt

# Install and run Ollama with Gemma 3
ollama pull gemma3:2b
ollama serve

# Run the interactive demo
python src/function_calling_demo.py
```

## Workshop Structure
1. **Environment Setup** - Install Ollama and configure Gemma 3
2. **Function Schema Design** - Define tool interfaces and JSON schemas
3. **Prompt Engineering** - Create effective function calling prompts
4. **Tool Implementation** - Build search, translation, and weather tools
5. **Dynamic Decision Logic** - Implement tool selection mechanisms
6. **Interactive Demo** - Test the complete system
7. **Extension Guide** - Add new tools and capabilities

## Expected Results
- **Local Privacy**: All processing happens on your machine
- **Response Time**: 2-5 seconds per query (depending on model size)
- **Accuracy**: 85%+ correct tool selection
- **Extensibility**: Easy addition of new functions
- **Memory Efficiency**: Runs on consumer hardware

## File Structure
```
gemma3-function-calling-workshop/
├── notebooks/           # Jupyter notebooks for interactive learning
│   ├── 01_setup_and_exploration.ipynb
│   ├── 02_function_schema_design.ipynb
│   ├── 03_prompt_engineering.ipynb
│   └── 04_tool_integration.ipynb
├── src/                 # Core implementation
│   ├── function_calling_demo.py    # Main demo script
│   ├── gemma_client.py            # Ollama/Gemma interface
│   ├── function_registry.py       # Tool management
│   ├── tools/                     # Individual tool implementations
│   │   ├── search_tool.py
│   │   ├── translation_tool.py
│   │   └── weather_tool.py
│   └── utils/
├── config/              # Configuration files
│   ├── functions.json   # Function schemas
│   └── settings.yaml    # API keys and settings
├── examples/            # Example prompts and responses
├── tests/               # Test cases
└── requirements.txt     # Python dependencies
```

## Technical Architecture

### Function Calling Flow
1. **User Input** → Natural language query
2. **Prompt Construction** → Add function schemas to context
3. **Model Inference** → Gemma 3 generates structured JSON response
4. **JSON Parsing** → Extract function name and parameters
5. **Tool Execution** → Execute the appropriate external API call
6. **Response Formatting** → Present results to user

### Key Components
- **Gemma Client**: Interface to Ollama for model inference
- **Function Registry**: Manages available tools and their schemas
- **Tool Implementations**: Individual modules for each external service
- **Response Parser**: Extracts and validates function calls from model output

## Workshop Highlights

### 🎯 What You'll Learn
- How to set up Gemma 3 locally with Ollama
- Function calling through prompt engineering (no native support needed)
- JSON schema design for tool interfaces
- Dynamic tool selection strategies
- Privacy-preserving AI assistant architecture
- Performance optimization for local inference

### 🔧 Technical Stack
- **Model**: Google Gemma 3 (2B/9B/27B)
- **Runtime**: Ollama
- **APIs**: Serper.dev, MyMemory, OpenWeatherMap
- **Language**: Python 3.8+
- **Libraries**: requests, json, yaml, ollama-python

### 📊 Comparison with Other Solutions
| Feature | This Workshop | OpenAI Functions | Anthropic Tools |
|---------|---------------|------------------|-----------------|
| Privacy | ✅ Local | ❌ Cloud | ❌ Cloud |
| Cost | ✅ Free | 💰 Pay-per-use | 💰 Pay-per-use |
| Customization | ✅ Full control | ⚠️ Limited | ⚠️ Limited |
| Latency | ⚠️ 2-5s | ✅ <1s | ✅ <1s |
| Model Size | ✅ 2B-27B | ❓ Unknown | ❓ Unknown |

## Getting Started

Choose your preferred learning path:

### 📓 Interactive Notebooks (Recommended for Learning)
1. `01_setup_and_exploration.ipynb` - Environment setup and model testing
2. `02_function_schema_design.ipynb` - Design tool interfaces
3. `03_prompt_engineering.ipynb` - Optimize function calling prompts
4. `04_tool_integration.ipynb` - Build and test complete system

### 🚀 Command Line Demo (Quick Start)
```bash
python src/function_calling_demo.py
```

### 🔧 Custom Implementation
Use the modular components to build your own function calling system.

## Next Steps
After completing this workshop, you'll be able to:
- Build custom AI assistants with external tool integration
- Extend the system with new functions and APIs
- Deploy the solution in production environments
- Optimize performance for your specific use cases

## Resources
- [Gemma 3 Documentation](https://ai.google.dev/gemma)
- [Ollama Documentation](https://ollama.ai/docs)
- [Function Calling Best Practices](https://ai.google.dev/gemma/docs/capabilities/function-calling)
- [Original Tutorial](https://pub.towardsai.net/dynamic-multi-function-calling-locally-with-gemma-3-and-ollam-07ddabc8f665)

---

🎯 **Ready to build your own AI assistant with function calling capabilities?** Let's get started!
