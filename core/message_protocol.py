"""
Agent Communication Protocol and Message Formats
Defines message format and communication standards between agents.
"""

from typing import Dict, Any, Optional, List
from datetime import datetime
import json
import hashlib


class MessageType:
    """Message type enumerations."""
    COMMAND = "command"
    ACK = "acknowledgment"
    ERROR = "error"
    RESULT = "result"
    DELEGATION = "delegation"
    ELECTION = "election"
    AUTH_CHALLENGE = "authentication_challenge"
    AUTH_RESPONSE = "authentication_response"


class MessageStatus:
    """Message status enumerations."""
    PENDING = "pending"
    SENT = "sent"
    DELIVERED = "delivered"
    ACKNOWLEDGED = "acknowledged"
    FAILED = "failed"
    TIMEOUT = "timeout"


class MessageProtocol:
    """Standardized message protocol for agent communication."""
    
    def __init__(self):
        self.message_id_counter = 0
    
    @staticmethod
    def generate_message_id() -> str:
        """Generate unique message ID."""
        message_id = MessageProtocol().message_id_counter
        MessageProtocol().message_id_counter += 1
        return f"msg_{datetime.now().strftime('%Y%m%d%H%M%S')}_{message_id}"
    
    @staticmethod
    def create_message(
        from_agent: str,
        to_agent: str,
        message_type: str,
        content: Dict[str, Any],
        priority: int = 1,
        metadata: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Create a standardized message.
        
        Args:
            from_agent: Source agent ID
            to_agent: Destination agent ID
            message_type: Type of message
            content: Message content payload
            priority: Priority level (1=highest, 5=lowest)
            metadata: Additional metadata
        
        Returns:
            Message dictionary
        """
        message = {
            "header": {
                "message_id": MessageProtocol.generate_message_id(),
                "timestamp": datetime.now().isoformat(),
                "version": "1.0",
                "priority": priority
            },
            "source": from_agent,
            "destination": to_agent,
            "type": message_type,
            "content": content,
            "status": MessageStatus.PENDING,
            "metadata": metadata or {}
        }
        return message
    
    @staticmethod
    def validate_message(message: Dict[str, Any]) -> bool:
        """Validate message structure and required fields."""
        required_fields = ["header", "source", "destination", "type", "content"]
        for field in required_fields:
            if field not in message:
                return False
        
        header_required = ["message_id", "timestamp", "priority"]
        for field in header_required:
            if field not in message["header"]:
                return False
        
        return True
    
    @staticmethod
    def get_message_type_info(message_type: str) -> Dict[str, Any]:
        """Get information about message type."""
        type_info = {
            MessageType.COMMAND: {
                "name": "Command",
                "direction": "downstream",
                "requires_response": True,
                "priority_range": (1, 3)
            },
            MessageType.ACK: {
                "name": "Acknowledgment",
                "direction": "upstream",
                "requires_response": False,
                "priority_range": (1, 5)
            },
            MessageType.ERROR: {
                "name": "Error Response",
                "direction": "upstream",
                "requires_response": False,
                "priority_range": (3, 5)
            },
            MessageType.RESULT: {
                "name": "Task Result",
                "direction": "upstream",
                "requires_response": False,
                "priority_range": (1, 2)
            },
            MessageType.DELEGATION: {
                "name": "Task Delegation",
                "direction": "downstream",
                "requires_response": True,
                "priority_range": (1, 3)
            },
            MessageType.ELECTION: {
                "name": "Election Message",
                "direction": "bidirectional",
                "requires_response": True,
                "priority_range": (1, 4)
            },
            MessageType.AUTH_CHALLENGE: {
                "name": "Authentication Challenge",
                "direction": "upstream",
                "requires_response": True,
                "priority_range": (1, 3)
            },
            MessageType.AUTH_RESPONSE: {
                "name": "Authentication Response",
                "direction": "upstream",
                "requires_response": False,
                "priority_range": (1, 4)
            }
        }
        return type_info.get(message_type, {
            "name": "Unknown",
            "direction": "bidirectional",
            "requires_response": False,
            "priority_range": (3, 5)
        })
    
    @staticmethod
    def encrypt_message(message: Dict[str, Any], key: str) -> Dict[str, Any]:
        """Encrypt message content (simplified for demo)."""
        import base64
        content_str = json.dumps(message["content"])
        encrypted = hashlib.sha256((content_str + key).encode()).hexdigest()
        message["encryped_content"] = encrypted
        return message
    
    @staticmethod
    def decrypt_message(message: Dict[str, Any], key: str) -> Dict[str, Any]:
        """Decrypt message content (simplified for demo)."""
        pass
    
    def marshal_message(self, message: Dict[str, Any]) -> bytes:
        """Convert message to bytes for transmission."""
        return json.dumps(message).encode('utf-8')
    
    def unmarshal_message(self, data: bytes) -> Optional[Dict[str, Any]]:
        """Convert bytes back to message dictionary."""
        try:
            return json.loads(data.decode('utf-8'))
        except Exception:
            return None


class CommandMessagePayload:
    """Payload for command messages."""
    
    @staticmethod
    def create(command: str, parameters: Dict[str, Any], task_id: str = "") -> Dict[str, Any]:
        return {
            "command": command,
            "parameters": parameters,
            "task_id": task_id,
            "context": {},
            "metadata": {}
        }


class ResultMessagePayload:
    """Payload for result messages."""
    
    @staticmethod
    def create(status: str, result: Any, task_id: str = "", error: Optional[str] = None) -> Dict[str, Any]:
        return {
            "status": status,
            "result": result,
            "task_id": task_id,
            "error": error,
            "execution_time": None
        }
