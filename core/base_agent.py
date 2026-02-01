"""
Base Agent Interface and Abstract Class
Defines common interface for all agents in the military hierarchy.
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Any, Optional
from datetime import datetime
import json
import uuid


class BaseAgent(ABC):
    """Abstract base class for all agents in the command hierarchy."""
    
    def __init__(self, agent_id: str, tier: int, name: str, capabilities: Dict[str, Any]):
        """
        Initialize base agent.
        
        Args:
            agent_id: Unique identifier for the agent
            tier: Command tier (1=General, 2=Lieutenant, 3=Squad)
            name: Human-readable name
            capabilities: Dictionary of agent capabilities
        """
        self.agent_id = agent_id or str(uuid.uuid4())
        self.tier = tier
        self.name = name
        self.capabilities = capabilities
        self.status = "active"
        self.active_tasks = []
        self.message_queue = []
        self.created_at = datetime.now()
        self.updated_at = datetime.now()
    
    @abstractmethod
    def execute_task(self, task: Dict[str, Any]) -> Dict[str, Any]:
        """Execute a task assigned by higher command."""
        pass
    
    @abstractmethod
    def receive_message(self, message: Dict[str, Any]) -> None:
        """Receive and process a command message."""
        pass
    
    def forward_message(self, target_agent: str, message: Dict[str, Any]) -> bool:
        """Forward a message to a target agent."""
        pass
    
    def authenticate(self, credentials: Dict[str, Any]) -> bool:
        """Authenticate with authentication system."""
        pass
    
    def authorize(self, action: str, resource: str) -> bool:
        """Check authorization for an action."""
        pass
    
    def get_status(self) -> Dict[str, Any]:
        """Get current agent status."""
        pass

class GeneralAgent(BaseAgent):
    """General Agent - Tier 1 (GLM-4.7-flash capable)."""
    
    def __init__(self, name: str, capabilities: Optional[Dict[str, Any]] = None):
        super().__init__(
            agent_id="",
            tier=1,
            name=name,
            capabilities=capabilities or {
                "model": "glm-4.7-flash",
                "max_tasks": 100,
                "delegation_capability": True,
                "strategic_planning": True
            }
        )
    
    def execute_task(self, task: Dict[str, Any]) -> Dict[str, Any]:
        """Execute tasks with strategic planning and delegation."""
        pass
    
    def delegate_to_lieutenant(self, lieutenant_id: str, task: Dict[str, Any]) -> bool:
        """Delegate task to a Lieutenant Agent."""
        pass

class LieutenantAgent(BaseAgent):
    """Lieutenant Agent - Tier 2 (M-series Mac mini capable)."""
    
    def __init__(self, name: str, capabilities: Optional[Dict[str, Any]] = None):
        super().__init__(
            agent_id="",
            tier=2,
            name=name,
            capabilities=capabilities or {
                "model": "m-series-mac-mini",
                "max_tasks": 50,
                "direct_command": True,
                "sub_delegation": False
            }
        )
    
    def execute_task(self, task: Dict[str, Any]) -> Dict[str, Any]:
        """Execute tasks as direct command."""
        pass
    
    def forward_to_tier3(self, tier3_id: str, task: Dict[str, Any]) -> bool:
        """Forward task to Tier 3 Agent."""
        pass

class Tier3Agent(BaseAgent):
    """Tier 3 Agent - Lower rank (Cheap hardware with remote LLM access)."""
    
    def __init__(self, name: str, capabilities: Optional[Dict[str, Any]] = None):
        super().__init__(
            agent_id="",
            tier=3,
            name=name,
            capabilities=capabilities or {
                "model": "local-hardware",
                "max_tasks": 10,
                "llm_access": True,
                "remote_llm_integration": True
            }
        )
    
    def execute_task(self, task: Dict[str, Any]) -> Dict[str, Any]:
        """Execute tasks using remote LLM or local hardware."""
        pass
    
    def query_remote_llm(self, prompt: str, context: Optional[str] = None) -> str:
        """Query external LLM for task completion."""
        pass
