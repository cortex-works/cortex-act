"""
Data preparation utilities for fine-tuning xLAM-2 on banking agent dataset.
"""

import json
import os
from typing import List, Dict, Any, Tuple
from datasets import load_dataset, Dataset
import pandas as pd
from transformers import AutoTokenizer


class BankingDatasetProcessor:
    """Process Galileo AI banking dataset for xLAM-2 fine-tuning."""
    
    def __init__(self, model_name: str = "Salesforce/xLAM-2-1b-fc-r"):
        self.model_name = model_name
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        self.domain = "banking"
        
    def load_raw_data(self) -> Tuple[List[Dict], List[Dict], List[Dict]]:
        """Load raw dataset components."""
        print("Loading dataset components...")
        
        tools = load_dataset("galileo-ai/agent-leaderboard-v2", "tools", split=self.domain)
        personas = load_dataset("galileo-ai/agent-leaderboard-v2", "personas", split=self.domain)
        scenarios = load_dataset("galileo-ai/agent-leaderboard-v2", "adaptive_tool_use", split=self.domain)
        
        # Convert tools with proper JSON parsing
        converted_tools = []
        for tool in tools:
            tool_dict = dict(tool)
            if 'properties' in tool_dict and isinstance(tool_dict['properties'], str):
                tool_dict['properties'] = json.loads(tool_dict['properties'])
            if 'response_schema' in tool_dict and isinstance(tool_dict['response_schema'], str):
                tool_dict['response_schema'] = json.loads(tool_dict['response_schema'])
            converted_tools.append(tool_dict)
        
        return converted_tools, list(personas), list(scenarios)
    
    def create_training_examples(self, scenarios: List[Dict], personas: List[Dict], 
                               tools: List[Dict]) -> List[Dict]:
        """Convert scenarios into training examples for xLAM-2."""
        training_examples = []
        
        # Create persona lookup
        persona_lookup = {p['persona_index']: p for p in personas}
        
        for scenario in scenarios:
            persona_idx = scenario['persona_index']
            persona = persona_lookup.get(persona_idx, {})
            
            # Create conversation format
            messages = [
                {
                    "role": "system",
                    "content": self._create_system_prompt(persona, tools)
                },
                {
                    "role": "user", 
                    "content": scenario['first_message']
                }
            ]
            
            # Create expected assistant response based on goals
            assistant_response = self._generate_assistant_response(
                scenario['user_goals'], tools
            )
            
            messages.append({
                "role": "assistant",
                "content": assistant_response
            })
            
            training_examples.append({
                "messages": messages,
                "tools": tools,
                "persona_index": persona_idx,
                "user_goals": scenario['user_goals'],
                "scenario_id": len(training_examples)
            })
        
        return training_examples
    
    def _create_system_prompt(self, persona: Dict, tools: List[Dict]) -> str:
        """Create system prompt for banking agent."""
        base_prompt = """You are a helpful banking assistant. You can help customers with various banking tasks including:
- Account management and transfers
- Credit card and loan services  
- International transactions and currency exchange
- Investment and savings products
- Dispute resolution and fraud reporting
- Branch and ATM location services

Always be professional, accurate, and helpful. Use the available tools to assist customers effectively."""
        
        if persona.get('communication_style'):
            base_prompt += f"\n\nCommunication style: {persona['communication_style']}"
        
        return base_prompt
    
    def _generate_assistant_response(self, goals: List[str], tools: List[Dict]) -> str:
        """Generate appropriate assistant response based on user goals."""
        # This is a simplified version - in practice, you'd want more sophisticated response generation
        response_parts = []
        
        response_parts.append("I'll help you with all of these banking matters. Let me address each of your requests:")
        
        for i, goal in enumerate(goals, 1):
            if "transfer" in goal.lower():
                response_parts.append(f"{i}. I'll process the transfer you mentioned.")
            elif "dispute" in goal.lower():
                response_parts.append(f"{i}. I'll help you dispute that transaction.")
            elif "card" in goal.lower() and ("lost" in goal.lower() or "stolen" in goal.lower()):
                response_parts.append(f"{i}. I'll report your card as lost/stolen and arrange a replacement.")
            elif "payment" in goal.lower() and "automatic" in goal.lower():
                response_parts.append(f"{i}. I'll set up automatic payments as requested.")
            elif "exchange" in goal.lower() or "rate" in goal.lower():
                response_parts.append(f"{i}. I'll check current exchange rates for you.")
            else:
                response_parts.append(f"{i}. I'll assist with: {goal}")
        
        response_parts.append("Let me start processing these requests for you.")
        
        return "\n".join(response_parts)
    
    def prepare_for_training(self, output_dir: str = "data/processed") -> str:
        """Prepare complete dataset for training."""
        os.makedirs(output_dir, exist_ok=True)
        
        # Load raw data
        tools, personas, scenarios = self.load_raw_data()
        
        # Create training examples
        training_examples = self.create_training_examples(scenarios, personas, tools)
        
        # Split data (80/10/10)
        total = len(training_examples)
        train_size = int(0.8 * total)
        val_size = int(0.1 * total)
        
        train_data = training_examples[:train_size]
        val_data = training_examples[train_size:train_size + val_size]
        test_data = training_examples[train_size + val_size:]
        
        # Save splits
        splits = {
            'train': train_data,
            'validation': val_data,
            'test': test_data
        }
        
        for split_name, split_data in splits.items():
            output_file = os.path.join(output_dir, f"{split_name}.json")
            with open(output_file, 'w') as f:
                json.dump(split_data, f, indent=2)
            print(f"Saved {len(split_data)} examples to {output_file}")
        
        # Save tools and personas separately
        with open(os.path.join(output_dir, "tools.json"), 'w') as f:
            json.dump(tools, f, indent=2)
        
        with open(os.path.join(output_dir, "personas.json"), 'w') as f:
            json.dump(personas, f, indent=2)
        
        print(f"Dataset preparation complete. Files saved to {output_dir}")
        return output_dir
    
    def analyze_dataset(self, scenarios: List[Dict]) -> Dict[str, Any]:
        """Analyze dataset characteristics."""
        analysis = {}
        
        # Goal distribution
        goal_counts = [len(s['user_goals']) for s in scenarios]
        analysis['goal_distribution'] = {
            'mean': sum(goal_counts) / len(goal_counts),
            'min': min(goal_counts),
            'max': max(goal_counts),
            'counts': {i: goal_counts.count(i) for i in set(goal_counts)}
        }
        
        # Message length analysis
        message_lengths = [len(s['first_message'].split()) for s in scenarios]
        analysis['message_lengths'] = {
            'mean': sum(message_lengths) / len(message_lengths),
            'min': min(message_lengths),
            'max': max(message_lengths)
        }
        
        return analysis


if __name__ == "__main__":
    processor = BankingDatasetProcessor()
    output_dir = processor.prepare_for_training()
    
    # Load and analyze
    _, _, scenarios = processor.load_raw_data()
    analysis = processor.analyze_dataset(scenarios)
    
    print("\nDataset Analysis:")
    print(f"Total scenarios: {len(scenarios)}")
    print(f"Average goals per scenario: {analysis['goal_distribution']['mean']:.2f}")
    print(f"Average message length: {analysis['message_lengths']['mean']:.1f} words")
    print(f"Goal count distribution: {analysis['goal_distribution']['counts']}")