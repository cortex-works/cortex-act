"""
Web search tool using Serper.dev API.
"""
import os
import requests
import logging
from typing import Dict, Any, List
from datetime import datetime

logger = logging.getLogger(__name__)


class SearchTool:
    """Tool for web search using Serper.dev API."""
    
    def __init__(self, config: Dict[str, Any]):
        """Initialize the search tool."""
        self.config = config
        self.api_key = os.getenv("SERPER_API_KEY")
        self.base_url = config.get("apis", {}).get("serper", {}).get("base_url", "https://google.serper.dev")
        self.enabled = config.get("apis", {}).get("serper", {}).get("enabled", True)
        
        if not self.api_key and self.enabled:
            logger.warning("SERPER_API_KEY not found in environment variables")
    
    def execute(self, parameters: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute web search.
        
        Args:
            parameters: Dict containing 'query' and optional 'num_results'
            
        Returns:
            Dict with search results
        """
        if not self.enabled:
            return {"error": "Search tool is disabled in configuration"}
        
        if not self.api_key:
            return {"error": "SERPER_API_KEY not configured"}
        
        query = parameters.get("query", "")
        num_results = parameters.get("num_results", 5)
        
        if not query:
            return {"error": "Query parameter is required"}
        
        try:
            # Prepare request
            headers = {
                "X-API-KEY": self.api_key,
                "Content-Type": "application/json"
            }
            
            payload = {
                "q": query,
                "num": min(num_results, 10)  # Max 10 results
            }
            
            # Make request to Serper API
            response = requests.post(
                f"{self.base_url}/search",
                json=payload,
                headers=headers,
                timeout=10
            )
            
            if response.status_code == 401:
                return {"error": "Invalid API key for Serper.dev"}
            
            response.raise_for_status()
            data = response.json()
            
            # Process results
            results = []
            organic_results = data.get("organic", [])
            
            for result in organic_results[:num_results]:
                results.append({
                    "title": result.get("title", ""),
                    "link": result.get("link", ""),
                    "snippet": result.get("snippet", ""),
                    "date": result.get("date", "")
                })
            
            # Add knowledge graph if available
            knowledge_graph = data.get("knowledgeGraph", {})
            
            # Add answer box if available
            answer_box = data.get("answerBox", {})
            
            return {
                "query": query,
                "results": results,
                "total_results": len(results),
                "knowledge_graph": knowledge_graph,
                "answer_box": answer_box,
                "search_time": datetime.now().isoformat(),
                "source": "serper.dev"
            }
            
        except requests.RequestException as e:
            logger.error(f"Error making search request: {e}")
            return {"error": f"Search request failed: {str(e)}"}
        except Exception as e:
            logger.error(f"Unexpected error in search: {e}")
            return {"error": f"Search failed: {str(e)}"}
    
    def get_status(self) -> Dict[str, Any]:
        """Get the status of the search tool."""
        status = {
            "name": "search_web",
            "enabled": self.enabled,
            "api_configured": bool(self.api_key),
            "base_url": self.base_url
        }
        
        if self.enabled and self.api_key:
            # Test API connectivity
            try:
                headers = {"X-API-KEY": self.api_key}
                response = requests.get(
                    f"{self.base_url}/search",
                    headers=headers,
                    timeout=5
                )
                status["api_accessible"] = True
                status["last_check"] = datetime.now().isoformat()
            except Exception as e:
                status["api_accessible"] = False
                status["error"] = str(e)
        
        return status
    
    def format_results_for_display(self, search_result: Dict[str, Any]) -> str:
        """Format search results for human-readable display."""
        if "error" in search_result:
            return f"❌ Search Error: {search_result['error']}"
        
        output = [f"🔍 Search Results for: '{search_result['query']}'"]
        output.append(f"Found {search_result['total_results']} results\n")
        
        # Add answer box if available
        if search_result.get("answer_box"):
            answer = search_result["answer_box"]
            if "answer" in answer:
                output.append(f"📋 Quick Answer: {answer['answer']}\n")
        
        # Add knowledge graph if available
        if search_result.get("knowledge_graph"):
            kg = search_result["knowledge_graph"]
            if "title" in kg:
                output.append(f"📚 Knowledge: {kg.get('title', '')} - {kg.get('description', '')}\n")
        
        # Add search results
        for i, result in enumerate(search_result.get("results", []), 1):
            output.append(f"{i}. **{result['title']}**")
            output.append(f"   {result['snippet']}")
            output.append(f"   🔗 {result['link']}")
            if result.get("date"):
                output.append(f"   📅 {result['date']}")
            output.append("")
        
        return "\n".join(output)


def main():
    """Test the search tool."""
    # Setup logging
    logging.basicConfig(level=logging.INFO)
    
    # Load config (simplified for testing)
    config = {
        "apis": {
            "serper": {
                "enabled": True,
                "base_url": "https://google.serper.dev"
            }
        }
    }
    
    # Initialize tool
    tool = SearchTool(config)
    
    # Test search
    result = tool.execute({
        "query": "latest AI developments 2024",
        "num_results": 3
    })
    
    # Display results
    if "error" in result:
        print(f"Error: {result['error']}")
    else:
        print(tool.format_results_for_display(result))


if __name__ == "__main__":
    main()
