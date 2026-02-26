"""
Translation tool using MyMemory API.
"""
import requests
import logging
from typing import Dict, Any
from datetime import datetime
import urllib.parse

logger = logging.getLogger(__name__)


class TranslationTool:
    """Tool for text translation using MyMemory API."""
    
    def __init__(self, config: Dict[str, Any]):
        """Initialize the translation tool."""
        self.config = config
        self.base_url = config.get("apis", {}).get("mymemory", {}).get("base_url", "https://api.mymemory.translated.net")
        self.enabled = config.get("apis", {}).get("mymemory", {}).get("enabled", True)
        
        # Language codes mapping
        self.language_codes = {
            "english": "en", "spanish": "es", "french": "fr", "german": "de",
            "italian": "it", "portuguese": "pt", "russian": "ru", "chinese": "zh",
            "japanese": "ja", "korean": "ko", "arabic": "ar", "hindi": "hi",
            "dutch": "nl", "swedish": "sv", "norwegian": "no", "danish": "da",
            "finnish": "fi", "polish": "pl", "czech": "cs", "hungarian": "hu",
            "greek": "el", "hebrew": "he", "turkish": "tr", "thai": "th",
            "vietnamese": "vi", "indonesian": "id", "malay": "ms", "filipino": "tl"
        }
    
    def execute(self, parameters: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute text translation.
        
        Args:
            parameters: Dict containing 'text', 'target_lang', and optional 'source_lang'
            
        Returns:
            Dict with translation result
        """
        if not self.enabled:
            return {"error": "Translation tool is disabled in configuration"}
        
        text = parameters.get("text", "")
        target_lang = parameters.get("target_lang", "")
        source_lang = parameters.get("source_lang", "auto")
        
        if not text:
            return {"error": "Text parameter is required"}
        
        if not target_lang:
            return {"error": "Target language parameter is required"}
        
        # Normalize language codes
        source_lang = self._normalize_language_code(source_lang)
        target_lang = self._normalize_language_code(target_lang)
        
        if len(text) > 500:
            return {"error": "Text too long (max 500 characters for free tier)"}
        
        try:
            # Prepare request
            lang_pair = f"{source_lang}|{target_lang}" if source_lang != "auto" else target_lang
            
            params = {
                "q": text,
                "langpair": lang_pair
            }
            
            # Make request to MyMemory API
            response = requests.get(
                f"{self.base_url}/get",
                params=params,
                timeout=10
            )
            
            response.raise_for_status()
            data = response.json()
            
            # Process response
            if data.get("responseStatus") != 200:
                return {
                    "error": f"Translation API error: {data.get('responseDetails', 'Unknown error')}"
                }
            
            response_data = data.get("responseData", {})
            translated_text = response_data.get("translatedText", "")
            
            # Check if translation was successful
            if not translated_text or translated_text.upper() == "PLEASE SELECT A VALID LANGUAGE PAIR":
                return {
                    "error": f"Invalid language pair: {source_lang} -> {target_lang}"
                }
            
            # Get detected source language if auto-detection was used
            detected_lang = source_lang
            if source_lang == "auto":
                # MyMemory doesn't always provide detected language info
                # We'll use the original source_lang parameter
                detected_lang = "auto-detected"
            
            return {
                "original_text": text,
                "translated_text": translated_text,
                "source_language": detected_lang,
                "target_language": target_lang,
                "confidence": response_data.get("match", 0),
                "translation_time": datetime.now().isoformat(),
                "source": "mymemory.translated.net"
            }
            
        except requests.RequestException as e:
            logger.error(f"Error making translation request: {e}")
            return {"error": f"Translation request failed: {str(e)}"}
        except Exception as e:
            logger.error(f"Unexpected error in translation: {e}")
            return {"error": f"Translation failed: {str(e)}"}
    
    def _normalize_language_code(self, lang: str) -> str:
        """Normalize language code or name to standard code."""
        if not lang:
            return "auto"
        
        lang = lang.lower().strip()
        
        # If it's already a code, return it
        if len(lang) == 2:
            return lang
        
        # If it's "auto" or similar
        if lang in ["auto", "detect", "automatic"]:
            return "auto"
        
        # Look up in language mapping
        return self.language_codes.get(lang, lang)
    
    def get_status(self) -> Dict[str, Any]:
        """Get the status of the translation tool."""
        status = {
            "name": "translate_text",
            "enabled": self.enabled,
            "base_url": self.base_url,
            "supported_languages": len(self.language_codes)
        }
        
        if self.enabled:
            # Test API connectivity
            try:
                response = requests.get(
                    f"{self.base_url}/get",
                    params={"q": "test", "langpair": "en|es"},
                    timeout=5
                )
                status["api_accessible"] = response.status_code == 200
                status["last_check"] = datetime.now().isoformat()
            except Exception as e:
                status["api_accessible"] = False
                status["error"] = str(e)
        
        return status
    
    def get_supported_languages(self) -> Dict[str, str]:
        """Get mapping of supported language names to codes."""
        return self.language_codes.copy()
    
    def format_result_for_display(self, translation_result: Dict[str, Any]) -> str:
        """Format translation result for human-readable display."""
        if "error" in translation_result:
            return f"❌ Translation Error: {translation_result['error']}"
        
        output = [
            "🌐 Translation Result:",
            f"📝 Original ({translation_result['source_language']}): {translation_result['original_text']}",
            f"🔄 Translated ({translation_result['target_language']}): {translation_result['translated_text']}"
        ]
        
        if translation_result.get("confidence"):
            confidence = translation_result["confidence"]
            confidence_emoji = "🟢" if confidence > 0.8 else "🟡" if confidence > 0.5 else "🔴"
            output.append(f"{confidence_emoji} Confidence: {confidence:.2f}")
        
        return "\n".join(output)


def main():
    """Test the translation tool."""
    # Setup logging
    logging.basicConfig(level=logging.INFO)
    
    # Load config (simplified for testing)
    config = {
        "apis": {
            "mymemory": {
                "enabled": True,
                "base_url": "https://api.mymemory.translated.net"
            }
        }
    }
    
    # Initialize tool
    tool = TranslationTool(config)
    
    # Test translation
    result = tool.execute({
        "text": "Hello, how are you today?",
        "source_lang": "en",
        "target_lang": "es"
    })
    
    # Display results
    print(tool.format_result_for_display(result))
    
    # Test with auto-detection
    result2 = tool.execute({
        "text": "Bonjour le monde",
        "target_lang": "english"
    })
    
    print("\n" + tool.format_result_for_display(result2))


if __name__ == "__main__":
    main()
