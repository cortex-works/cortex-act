"""
Gemma 3 client for function calling with Ollama.
"""
import json
import re
import logging
from typing import Dict, List, Any, Optional, Tuple
import requests
import yaml
from pathlib import Path

logger = logging.getLogger(__name__)


class Gemma3Client:
    """Client for interacting with Gemma 3 via Ollama with function calling capabilities."""
    
    def __init__(self, config_path: str = "config/gemma3_config.yaml"):
        """Initialize the Gemma 3 client."""
        self.config = self._load_config(config_path)
        self.ollama_url = f"http://{self.config['ollama']['host']}:{self.config['ollama']['port']}"
        self.functions = self._load_functions()
        
        # Test connection
        self._test_connection()
        
    def _load_config(self, config_path: str) -> Dict[str, Any]:
        """Load configuration from YAML file."""
        try:
            with open(config_path, 'r') as f:
                return yaml.safe_load(f)
        except FileNotFoundError:
            logger.error(f"Config file not found: {config_path}")
            raise
        except yaml.YAMLError as e:
            logger.error(f"Error parsing config file: {e}")
            raise
    
    def _load_functions(self) -> Dict[str, Any]:
        """Load function definitions from JSON file."""
        try:
            with open("config/functions.json", 'r') as f:
                return json.load(f)
        except FileNotFoundError:
            logger.error("Functions file not found: config/functions.json")
            raise
        except json.JSONDecodeError as e:
            logger.error(f"Error parsing functions file: {e}")
            raise
    
    def _test_connection(self):
        """Test connection to Ollama."""
        try:
            response = requests.get(f"{self.ollama_url}/api/tags", timeout=5)
            response.raise_for_status()
            logger.info("Successfully connected to Ollama")
        except requests.RequestException as e:
            logger.error(f"Failed to connect to Ollama: {e}")
            raise
    
    def _build_system_prompt(self) -> str:
        """Build the system prompt with function definitions."""
        function_list = []
        for func in self.functions["functions"]:
            func_desc = f"- {func['name']}: {func['description']}"
            function_list.append(func_desc)
        
        function_list_str = "\n".join(function_list)
        return self.functions["system_prompt_template"].format(function_list=function_list_str)
    
    def _extract_function_call(self, response: str) -> Tuple[Optional[Dict], str]:
        """
        Extract function call from model response.
        Returns: (function_call_dict, remaining_text)
        """
        # Look for JSON block in response
        json_pattern = r'```json\s*(\{.*?\})\s*```'
        match = re.search(json_pattern, response, re.DOTALL)
        
        if match:
            try:
                function_data = json.loads(match.group(1))
                if "function_call" in function_data:
                    # Remove the JSON block from response
                    clean_response = re.sub(json_pattern, "", response, flags=re.DOTALL).strip()
                    return function_data["function_call"], clean_response
            except json.JSONDecodeError as e:
                logger.warning(f"Failed to parse function call JSON: {e}")
        
        # Alternative: Look for direct function call format
        func_pattern = r'\{"function_call":\s*\{[^}]+\}[^}]*\}'
        match = re.search(func_pattern, response)
        if match:
            try:
                function_data = json.loads(match.group(0))
                clean_response = response.replace(match.group(0), "").strip()
                return function_data["function_call"], clean_response
            except json.JSONDecodeError as e:
                logger.warning(f"Failed to parse alternative function call JSON: {e}")
        
        return None, response
    
    def _validate_function_call(self, function_call: Dict[str, Any]) -> bool:
        """Validate that the function call matches available functions."""
        if "name" not in function_call or "parameters" not in function_call:
            return False
        
        function_name = function_call["name"]
        
        # Find function definition
        func_def = None
        for func in self.functions["functions"]:
            if func["name"] == function_name:
                func_def = func
                break
        
        if not func_def:
            logger.warning(f"Unknown function: {function_name}")
            return False
        
        # Basic parameter validation
        required_params = func_def["parameters"].get("required", [])
        provided_params = function_call["parameters"].keys()
        
        for param in required_params:
            if param not in provided_params:
                logger.warning(f"Missing required parameter: {param}")
                return False
        
        return True
    
    def generate(self, messages: List[Dict[str, str]], 
                 include_functions: bool = True) -> Dict[str, Any]:
        """
        Generate response from Gemma 3 with optional function calling.
        
        Args:
            messages: List of message dicts with 'role' and 'content'
            include_functions: Whether to include function calling capabilities
            
        Returns:
            Dict with 'response', 'function_call', and metadata
        """
        # Build prompt
        if include_functions:
            system_prompt = self._build_system_prompt()
            full_messages = [{"role": "system", "content": system_prompt}] + messages
        else:
            full_messages = messages
        
        # Convert to prompt string (simple approach for Ollama)
        prompt_parts = []
        for msg in full_messages:
            role = msg["role"]
            content = msg["content"]
            if role == "system":
                prompt_parts.append(f"System: {content}")
            elif role == "user":
                prompt_parts.append(f"User: {content}")
            elif role == "assistant":
                prompt_parts.append(f"Assistant: {content}")
        
        prompt_parts.append("Assistant:")
        prompt = "\n\n".join(prompt_parts)
        
        # Make request to Ollama
        payload = {
            "model": self.config["model"]["name"],
            "prompt": prompt,
            "stream": False,
            "options": {
                "temperature": self.config["model"]["temperature"],
                "top_p": self.config["model"]["top_p"],
                "num_predict": self.config["model"]["max_tokens"],
                "stop": self.config["model"]["stop_sequences"]
            }
        }
        
        try:
            response = requests.post(
                f"{self.ollama_url}/api/generate",
                json=payload,
                timeout=self.config["ollama"]["timeout"]
            )
            response.raise_for_status()
            result = response.json()
            
            raw_response = result.get("response", "")
            
            # Extract function call if present
            function_call, clean_response = self._extract_function_call(raw_response)
            
            # Validate function call
            if function_call and not self._validate_function_call(function_call):
                function_call = None
                clean_response = raw_response  # Fall back to original response
            
            return {
                "response": clean_response.strip(),
                "function_call": function_call,
                "raw_response": raw_response,
                "model": self.config["model"]["name"],
                "tokens_used": result.get("eval_count", 0),
                "generation_time": result.get("eval_duration", 0) / 1e9  # Convert to seconds
            }
            
        except requests.RequestException as e:
            logger.error(f"Error calling Ollama: {e}")
            raise
        except json.JSONDecodeError as e:
            logger.error(f"Error parsing Ollama response: {e}")
            raise
    
    def get_available_functions(self) -> List[Dict[str, Any]]:
        """Get list of available functions."""
        return self.functions["functions"]
    
    def get_function_schema(self, function_name: str) -> Optional[Dict[str, Any]]:
        """Get schema for a specific function."""
        for func in self.functions["functions"]:
            if func["name"] == function_name:
                return func
        return None


def main():
    """Test the Gemma 3 client."""
    # Setup logging
    logging.basicConfig(level=logging.INFO)
    
    # Initialize client
    client = Gemma3Client()
    
    # Test basic generation
    messages = [
        {"role": "user", "content": "What's the weather like in San Francisco?"}
    ]
    
    result = client.generate(messages)
    
    print("Response:", result["response"])
    if result["function_call"]:
        print("Function Call:", json.dumps(result["function_call"], indent=2))
    
    print(f"Tokens used: {result['tokens_used']}")
    print(f"Generation time: {result['generation_time']:.2f}s")


if __name__ == "__main__":
    main()
