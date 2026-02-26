#!/usr/bin/env python3
"""
Interactive demo for the fine-tuned banking agent.
"""

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import PeftModel
import json
import os


class BankingAgentDemo:
    """Interactive demo for banking agent."""
    
    def __init__(self, model_path: str = "models/xlam-banking-agent"):
        self.model_path = model_path
        self.model = None
        self.tokenizer = None
        self.tools = []
        self.load_model_and_tools()
    
    def load_model_and_tools(self):
        """Load model, tokenizer, and tools."""
        print("Loading banking agent model...")
        
        # Load tokenizer
        self.tokenizer = AutoTokenizer.from_pretrained(self.model_path)
        if self.tokenizer.pad_token is None:
            self.tokenizer.pad_token = self.tokenizer.eos_token
        
        # Load base model
        base_model = AutoModelForCausalLM.from_pretrained(
            "Salesforce/xLAM-2-1b-fc-r",
            torch_dtype=torch.bfloat16,
            device_map="auto"
        )
        
        # Load LoRA adapter if exists
        if os.path.exists(os.path.join(self.model_path, "adapter_config.json")):
            self.model = PeftModel.from_pretrained(base_model, self.model_path)
        else:
            self.model = base_model
        
        self.model.eval()
        
        # Load tools
        tools_path = "data/processed/tools.json"
        if os.path.exists(tools_path):
            with open(tools_path, 'r') as f:
                self.tools = json.load(f)
        
        print("Model loaded successfully!")
    
    def generate_response(self, user_message: str, conversation_history: list = None) -> str:
        """Generate response to user message."""
        # Build conversation
        messages = [
            {
                "role": "system",
                "content": """You are a helpful banking assistant. You can help customers with:
- Account management and transfers
- Credit card and loan services
- International transactions and currency exchange
- Investment and savings products
- Dispute resolution and fraud reporting
- Branch and ATM location services

Always be professional, accurate, and helpful."""
            }
        ]
        
        # Add conversation history
        if conversation_history:
            messages.extend(conversation_history)
        
        # Add current user message
        messages.append({"role": "user", "content": user_message})
        
        # Apply chat template
        prompt = self.tokenizer.apply_chat_template(
            messages,
            tools=self.tools,
            add_generation_prompt=True,
            tokenize=False
        )
        
        # Tokenize and generate
        inputs = self.tokenizer(
            prompt,
            return_tensors="pt",
            truncation=True,
            max_length=2048
        ).to(self.model.device)
        
        with torch.no_grad():
            outputs = self.model.generate(
                **inputs,
                max_new_tokens=512,
                do_sample=True,
                temperature=0.7,
                top_p=0.9,
                pad_token_id=self.tokenizer.eos_token_id
            )
        
        # Decode response
        response = self.tokenizer.decode(
            outputs[0][inputs['input_ids'].shape[1]:],
            skip_special_tokens=True
        )
        
        return response.strip()
    
    def run_demo(self):
        """Run interactive demo."""
        print("\n" + "="*60)
        print("🏦 Banking Agent Demo")
        print("="*60)
        print("Type 'quit' to exit, 'clear' to clear conversation history")
        print("Try asking about transfers, disputes, cards, loans, etc.")
        print("="*60 + "\n")
        
        conversation_history = []
        
        # Sample scenarios for quick testing
        sample_scenarios = [
            "I need to transfer $500 from savings to checking",
            "My credit card was stolen yesterday, what should I do?",
            "I want to dispute a charge of $299 from yesterday",
            "Can you help me set up automatic bill payments?",
            "What's the current exchange rate for Euros?",
            "I need to find an ATM near downtown"
        ]
        
        print("Sample scenarios you can try:")
        for i, scenario in enumerate(sample_scenarios, 1):
            print(f"{i}. {scenario}")
        print()
        
        while True:
            try:
                user_input = input("You: ").strip()
                
                if user_input.lower() == 'quit':
                    print("Thank you for using the Banking Agent Demo!")
                    break
                
                if user_input.lower() == 'clear':
                    conversation_history = []
                    print("Conversation history cleared.")
                    continue
                
                if not user_input:
                    continue
                
                # Check if user entered a number for sample scenario
                if user_input.isdigit():
                    scenario_idx = int(user_input) - 1
                    if 0 <= scenario_idx < len(sample_scenarios):
                        user_input = sample_scenarios[scenario_idx]
                        print(f"You: {user_input}")
                
                # Generate response
                print("Agent: ", end="", flush=True)
                response = self.generate_response(user_input, conversation_history)
                print(response)
                
                # Update conversation history
                conversation_history.extend([
                    {"role": "user", "content": user_input},
                    {"role": "assistant", "content": response}
                ])
                
                # Keep conversation history manageable
                if len(conversation_history) > 10:
                    conversation_history = conversation_history[-10:]
                
                print()
                
            except KeyboardInterrupt:
                print("\n\nGoodbye!")
                break
            except Exception as e:
                print(f"Error: {e}")
                continue


def main():
    """Main demo function."""
    demo = BankingAgentDemo()
    demo.run_demo()


if __name__ == "__main__":
    main()