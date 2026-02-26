"""
Function registry for managing and executing tools.
"""
import json
import logging
import os
from typing import Dict, Any, Optional, List
from datetime import datetime
import importlib
from pathlib import Path

logger = logging.getLogger(__name__)


class FunctionRegistry:
    """Registry for managing and executing function calls."""
    
    def __init__(self, config_path: str = "config/gemma3_config.yaml"):
        """Initialize the function registry."""
        self.tools = {}
        self.config = self._load_config(config_path)
        self._load_tools()
    
    def _load_config(self, config_path: str) -> Dict[str, Any]:
        """Load configuration."""
        import yaml
        try:
            with open(config_path, 'r') as f:
                return yaml.safe_load(f)
        except FileNotFoundError:
            logger.warning(f"Config file not found: {config_path}")
            return {}
    
    def _load_tools(self):
        """Load all available tools."""
        # Import and register tools
        try:
            from tools.search_tool import SearchTool
            self.tools["search_web"] = SearchTool(self.config)
        except ImportError as e:
            logger.warning(f"Failed to load search tool: {e}")
        
        try:
            from tools.translation_tool import TranslationTool
            self.tools["translate_text"] = TranslationTool(self.config)
        except ImportError as e:
            logger.warning(f"Failed to load translation tool: {e}")
        
        try:
            from tools.weather_tool import WeatherTool
            self.tools["get_weather"] = WeatherTool(self.config)
        except ImportError as e:
            logger.warning(f"Failed to load weather tool: {e}")
        
        try:
            from tools.math_tool import MathTool
            self.tools["calculate_math"] = MathTool(self.config)
        except ImportError as e:
            logger.warning(f"Failed to load math tool: {e}")
        
        try:
            from tools.time_tool import TimeTool
            self.tools["get_time_info"] = TimeTool(self.config)
        except ImportError as e:
            logger.warning(f"Failed to load time tool: {e}")
        
        logger.info(f"Loaded {len(self.tools)} tools: {list(self.tools.keys())}")
    
    def execute_function(self, function_name: str, parameters: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute a function call.
        
        Args:
            function_name: Name of the function to execute
            parameters: Parameters to pass to the function
            
        Returns:
            Dict with execution result
        """
        if function_name not in self.tools:
            return {
                "success": False,
                "error": f"Unknown function: {function_name}",
                "available_functions": list(self.tools.keys())
            }
        
        try:
            tool = self.tools[function_name]
            result = tool.execute(parameters)
            
            return {
                "success": True,
                "function_name": function_name,
                "parameters": parameters,
                "result": result,
                "timestamp": datetime.now().isoformat()
            }
            
        except Exception as e:
            logger.error(f"Error executing {function_name}: {e}")
            return {
                "success": False,
                "function_name": function_name,
                "parameters": parameters,
                "error": str(e),
                "timestamp": datetime.now().isoformat()
            }
    
    def get_available_functions(self) -> List[str]:
        """Get list of available function names."""
        return list(self.tools.keys())
    
    def get_tool_status(self) -> Dict[str, Any]:
        """Get status of all tools."""
        status = {}
        for name, tool in self.tools.items():
            try:
                tool_status = tool.get_status() if hasattr(tool, 'get_status') else {"status": "unknown"}
                status[name] = tool_status
            except Exception as e:
                status[name] = {"status": "error", "error": str(e)}
        
        return status
    
    def test_all_tools(self) -> Dict[str, Any]:
        """Test all tools with sample data."""
        test_results = {}
        
        # Test search tool
        if "search_web" in self.tools:
            try:
                result = self.execute_function("search_web", {"query": "test search", "num_results": 1})
                test_results["search_web"] = {"passed": result["success"], "details": result}
            except Exception as e:
                test_results["search_web"] = {"passed": False, "error": str(e)}
        
        # Test translation tool
        if "translate_text" in self.tools:
            try:
                result = self.execute_function("translate_text", {
                    "text": "Hello world",
                    "target_lang": "es"
                })
                test_results["translate_text"] = {"passed": result["success"], "details": result}
            except Exception as e:
                test_results["translate_text"] = {"passed": False, "error": str(e)}
        
        # Test weather tool
        if "get_weather" in self.tools:
            try:
                result = self.execute_function("get_weather", {"location": "London"})
                test_results["get_weather"] = {"passed": result["success"], "details": result}
            except Exception as e:
                test_results["get_weather"] = {"passed": False, "error": str(e)}
        
        # Test math tool
        if "calculate_math" in self.tools:
            try:
                result = self.execute_function("calculate_math", {"expression": "2 + 2"})
                test_results["calculate_math"] = {"passed": result["success"], "details": result}
            except Exception as e:
                test_results["calculate_math"] = {"passed": False, "error": str(e)}
        
        # Test time tool
        if "get_time_info" in self.tools:
            try:
                result = self.execute_function("get_time_info", {})
                test_results["get_time_info"] = {"passed": result["success"], "details": result}
            except Exception as e:
                test_results["get_time_info"] = {"passed": False, "error": str(e)}
        
        return test_results


class BaseTool:
    """Base class for all tools."""
    
    def __init__(self, config: Dict[str, Any]):
        """Initialize the tool with configuration."""
        self.config = config
        self.name = self.__class__.__name__
    
    def execute(self, parameters: Dict[str, Any]) -> Any:
        """Execute the tool with given parameters."""
        raise NotImplementedError("Subclasses must implement execute method")
    
    def get_status(self) -> Dict[str, Any]:
        """Get the status of this tool."""
        return {"status": "active", "name": self.name}
    
    def validate_parameters(self, parameters: Dict[str, Any], required: List[str]) -> bool:
        """Validate that required parameters are present."""
        for param in required:
            if param not in parameters:
                raise ValueError(f"Missing required parameter: {param}")
        return True


def main():
    """Test the function registry."""
    # Setup logging
    logging.basicConfig(level=logging.INFO)
    
    # Initialize registry
    registry = FunctionRegistry()
    
    # Show available functions
    print("Available functions:", registry.get_available_functions())
    
    # Test all tools
    test_results = registry.test_all_tools()
    
    print("\nTool test results:")
    for tool_name, result in test_results.items():
        status = "✅ PASSED" if result["passed"] else "❌ FAILED"
        print(f"{tool_name}: {status}")
        if not result["passed"]:
            print(f"  Error: {result.get('error', 'Unknown error')}")


if __name__ == "__main__":
    main()
