"""
Fine-tuning script for xLAM-2 on banking agent dataset.
"""

import json
import os
import torch
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    TrainingArguments,
    Trainer,
    DataCollatorForSeq2Seq
)
from datasets import Dataset
from peft import LoraConfig, get_peft_model, TaskType
import wandb


@dataclass
class ModelArguments:
    model_name_or_path: str = field(default="Salesforce/xLAM-2-1b-fc-r")
    use_lora: bool = field(default=True)
    lora_r: int = field(default=16)
    lora_alpha: int = field(default=32)
    lora_dropout: float = field(default=0.1)


@dataclass
class DataArguments:
    data_path: str = field(default="data/processed")
    max_length: int = field(default=2048)


class BankingAgentTrainer:
    """Trainer for banking agent fine-tuning."""
    
    def __init__(self, model_args: ModelArguments, data_args: DataArguments):
        self.model_args = model_args
        self.data_args = data_args
        self.tokenizer = None
        self.model = None
        
    def setup_model_and_tokenizer(self):
        """Initialize model and tokenizer."""
        print(f"Loading model: {self.model_args.model_name_or_path}")
        
        # Load tokenizer
        self.tokenizer = AutoTokenizer.from_pretrained(
            self.model_args.model_name_or_path,
            trust_remote_code=True
        )
        
        # Ensure pad token is set
        if self.tokenizer.pad_token is None:
            self.tokenizer.pad_token = self.tokenizer.eos_token
        
        # Load model
        self.model = AutoModelForCausalLM.from_pretrained(
            self.model_args.model_name_or_path,
            torch_dtype=torch.bfloat16,
            device_map="auto",
            trust_remote_code=True
        )
        
        # Apply LoRA if specified
        if self.model_args.use_lora:
            print("Applying LoRA configuration...")
            lora_config = LoraConfig(
                task_type=TaskType.CAUSAL_LM,
                r=self.model_args.lora_r,
                lora_alpha=self.model_args.lora_alpha,
                lora_dropout=self.model_args.lora_dropout,
                target_modules=["q_proj", "v_proj", "k_proj", "o_proj", "gate_proj", "up_proj", "down_proj"]
            )
            self.model = get_peft_model(self.model, lora_config)
            self.model.print_trainable_parameters()
    
    def load_dataset(self) -> Dict[str, Dataset]:
        """Load and process training datasets."""
        datasets = {}
        
        for split in ['train', 'validation']:
            file_path = os.path.join(self.data_args.data_path, f"{split}.json")
            
            with open(file_path, 'r') as f:
                data = json.load(f)
            
            # Process data for training
            processed_data = []
            for example in data:
                # Apply chat template
                formatted_text = self.tokenizer.apply_chat_template(
                    example['messages'],
                    tools=example.get('tools', []),
                    tokenize=False,
                    add_generation_prompt=False
                )
                
                processed_data.append({
                    'text': formatted_text,
                    'scenario_id': example['scenario_id'],
                    'persona_index': example['persona_index']
                })
            
            datasets[split] = Dataset.from_list(processed_data)
            print(f"Loaded {len(datasets[split])} examples for {split}")
        
        return datasets
    
    def tokenize_function(self, examples):
        """Tokenize examples for training."""
        # Tokenize the text
        tokenized = self.tokenizer(
            examples['text'],
            truncation=True,
            padding=False,
            max_length=self.data_args.max_length,
            return_tensors=None
        )
        
        # For causal LM, labels are the same as input_ids
        tokenized['labels'] = tokenized['input_ids'].copy()
        
        return tokenized
    
    def train(self, output_dir: str = "models/xlam-banking-agent"):
        """Run the training process."""
        # Setup model and tokenizer
        self.setup_model_and_tokenizer()
        
        # Load datasets
        datasets = self.load_dataset()
        
        # Tokenize datasets
        tokenized_datasets = {}
        for split, dataset in datasets.items():
            tokenized_datasets[split] = dataset.map(
                self.tokenize_function,
                batched=True,
                remove_columns=dataset.column_names
            )
        
        # Training arguments
        training_args = TrainingArguments(
            output_dir=output_dir,
            num_train_epochs=3,
            per_device_train_batch_size=4,
            per_device_eval_batch_size=4,
            gradient_accumulation_steps=4,
            warmup_steps=100,
            learning_rate=2e-4,
            fp16=False,
            bf16=True,
            logging_steps=10,
            evaluation_strategy="steps",
            eval_steps=100,
            save_steps=500,
            save_total_limit=3,
            load_best_model_at_end=True,
            metric_for_best_model="eval_loss",
            greater_is_better=False,
            report_to="wandb",
            run_name="xlam-banking-agent-finetune",
            dataloader_pin_memory=False,
            remove_unused_columns=False
        )
        
        # Data collator
        data_collator = DataCollatorForSeq2Seq(
            tokenizer=self.tokenizer,
            model=self.model,
            padding=True,
            return_tensors="pt"
        )
        
        # Initialize trainer
        trainer = Trainer(
            model=self.model,
            args=training_args,
            train_dataset=tokenized_datasets['train'],
            eval_dataset=tokenized_datasets['validation'],
            tokenizer=self.tokenizer,
            data_collator=data_collator
        )
        
        # Start training
        print("Starting training...")
        trainer.train()
        
        # Save final model
        trainer.save_model()
        self.tokenizer.save_pretrained(output_dir)
        
        print(f"Training completed. Model saved to {output_dir}")
        
        return trainer


def main():
    """Main training function."""
    # Initialize wandb
    wandb.init(project="xlam-banking-agent", name="finetune-run")
    
    # Setup arguments
    model_args = ModelArguments()
    data_args = DataArguments()
    
    # Create trainer and run training
    trainer = BankingAgentTrainer(model_args, data_args)
    trainer.train()


if __name__ == "__main__":
    main()