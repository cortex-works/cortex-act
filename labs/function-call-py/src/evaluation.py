"""
Evaluation framework for fine-tuned banking agent model.
"""

import json
import os
import torch
from typing import List, Dict, Any, Tuple
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import PeftModel
import pandas as pd
from tqdm import tqdm
import re


class BankingAgentEvaluator:
    """Evaluate banking agent performance on test scenarios."""
    
    def __init__(self, model_path: str, base_model: str = "Salesforce/xLAM-2-1b-fc-r"):
        self.model_path = model_path
        self.base_model = base_model
        self.model = None
        self.tokenizer = None
        self.load_model()
    
    def load_model(self):
        """Load the fine-tuned model."""
        print(f"Loading model from {self.model_path}")
        
        # Load tokenizer
        self.tokenizer = AutoTokenizer.from_pretrained(self.model_path)
        if self.tokenizer.pad_token is None:
            self.tokenizer.pad_token = self.tokenizer.eos_token
        
        # Load base model
        base_model = AutoModelForCausalLM.from_pretrained(
            self.base_model,
            torch_dtype=torch.bfloat16,
            device_map="auto"
        )
        
        # Load LoRA weights if they exist
        if os.path.exists(os.path.join(self.model_path, "adapter_config.json")):
            print("Loading LoRA adapter...")
            self.model = PeftModel.from_pretrained(base_model, self.model_path)
        else:
            self.model = base_model
        
        self.model.eval()
    
    def generate_response(self, messages: List[Dict], tools: List[Dict] = None, 
                         max_new_tokens: int = 512) -> str:
        """Generate response for given messages."""
        # Apply chat template
        prompt = self.tokenizer.apply_chat_template(
            messages,
            tools=tools or [],
            add_generation_prompt=True,
            tokenize=False
        )
        
        # Tokenize
        inputs = self.tokenizer(
            prompt,
            return_tensors="pt",
            truncation=True,
            max_length=2048
        ).to(self.model.device)
        
        # Generate
        with torch.no_grad():
            outputs = self.model.generate(
                **inputs,
                max_new_tokens=max_new_tokens,
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
    
    def evaluate_action_completion(self, response: str, user_goals: List[str]) -> Dict[str, Any]:
        """Evaluate Action Completion (AC) metric."""
        # Simple keyword-based evaluation
        # In practice, you'd want more sophisticated evaluation
        
        completed_goals = 0
        goal_analysis = []
        
        response_lower = response.lower()
        
        for goal in user_goals:
            goal_lower = goal.lower()
            
            # Check for key action words
            action_indicators = [
                "transfer", "dispute", "report", "set up", "check", "find",
                "update", "schedule", "convert", "exchange", "verify"
            ]
            
            goal_completed = False
            for indicator in action_indicators:
                if indicator in goal_lower and indicator in response_lower:
                    goal_completed = True
                    break
            
            if goal_completed:
                completed_goals += 1
            
            goal_analysis.append({
                "goal": goal,
                "completed": goal_completed
            })
        
        ac_score = completed_goals / len(user_goals) if user_goals else 0
        
        return {
            "ac_score": ac_score,
            "completed_goals": completed_goals,
            "total_goals": len(user_goals),
            "goal_analysis": goal_analysis
        }
    
    def evaluate_tool_selection_quality(self, response: str, expected_tools: List[str] = None) -> Dict[str, Any]:
        """Evaluate Tool Selection Quality (TSQ) metric."""
        # Extract potential tool calls from response
        # This is a simplified version - you'd want more sophisticated parsing
        
        tool_patterns = [
            r"transfer.*money|transfer.*funds",
            r"dispute.*transaction|dispute.*charge",
            r"report.*card|report.*lost|report.*stolen",
            r"set.*automatic|automatic.*payment",
            r"check.*balance|verify.*payment",
            r"find.*branch|locate.*atm",
            r"exchange.*rate|currency.*conversion"
        ]
        
        identified_tools = []
        for pattern in tool_patterns:
            if re.search(pattern, response.lower()):
                identified_tools.append(pattern.split('|')[0])
        
        # Simple TSQ calculation
        tsq_score = len(identified_tools) / max(len(expected_tools or []), 1)
        tsq_score = min(tsq_score, 1.0)  # Cap at 1.0
        
        return {
            "tsq_score": tsq_score,
            "identified_tools": identified_tools,
            "expected_tools": expected_tools or []
        }
    
    def evaluate_test_set(self, test_data_path: str) -> Dict[str, Any]:
        """Evaluate model on complete test set."""
        # Load test data
        with open(test_data_path, 'r') as f:
            test_data = json.load(f)
        
        # Load tools
        tools_path = os.path.join(os.path.dirname(test_data_path), "tools.json")
        with open(tools_path, 'r') as f:
            tools = json.load(f)
        
        results = []
        
        print(f"Evaluating {len(test_data)} test examples...")
        
        for example in tqdm(test_data):
            # Create messages for evaluation
            messages = [
                {"role": "system", "content": "You are a helpful banking assistant."},
                {"role": "user", "content": example['messages'][1]['content']}  # First user message
            ]
            
            # Generate response
            response = self.generate_response(messages, tools)
            
            # Evaluate metrics
            ac_eval = self.evaluate_action_completion(response, example['user_goals'])
            tsq_eval = self.evaluate_tool_selection_quality(response)
            
            result = {
                "scenario_id": example['scenario_id'],
                "persona_index": example['persona_index'],
                "user_goals": example['user_goals'],
                "generated_response": response,
                "ac_score": ac_eval['ac_score'],
                "tsq_score": tsq_eval['tsq_score'],
                "completed_goals": ac_eval['completed_goals'],
                "total_goals": ac_eval['total_goals']
            }
            
            results.append(result)
        
        # Calculate overall metrics
        overall_ac = sum(r['ac_score'] for r in results) / len(results)
        overall_tsq = sum(r['tsq_score'] for r in results) / len(results)
        
        evaluation_summary = {
            "overall_ac_score": overall_ac,
            "overall_tsq_score": overall_tsq,
            "total_scenarios": len(results),
            "detailed_results": results
        }
        
        return evaluation_summary
    
    def save_evaluation_results(self, results: Dict[str, Any], output_path: str):
        """Save evaluation results to file."""
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        
        with open(output_path, 'w') as f:
            json.dump(results, f, indent=2)
        
        # Also save summary CSV
        csv_path = output_path.replace('.json', '_summary.csv')
        df = pd.DataFrame(results['detailed_results'])
        df.to_csv(csv_path, index=False)
        
        print(f"Evaluation results saved to {output_path}")
        print(f"Summary CSV saved to {csv_path}")
        
        # Print summary
        print(f"\nEvaluation Summary:")
        print(f"Overall AC Score: {results['overall_ac_score']:.3f}")
        print(f"Overall TSQ Score: {results['overall_tsq_score']:.3f}")
        print(f"Total Scenarios: {results['total_scenarios']}")


def main():
    """Main evaluation function."""
    model_path = "models/xlam-banking-agent"
    test_data_path = "data/processed/test.json"
    output_path = "results/evaluation_results.json"
    
    # Create evaluator
    evaluator = BankingAgentEvaluator(model_path)
    
    # Run evaluation
    results = evaluator.evaluate_test_set(test_data_path)
    
    # Save results
    evaluator.save_evaluation_results(results, output_path)


if __name__ == "__main__":
    main()