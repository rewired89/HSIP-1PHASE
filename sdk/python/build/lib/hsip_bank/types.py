"""
Message types and data structures for HSIP
"""

from dataclasses import dataclass
from datetime import datetime
from typing import Optional, Dict, Any
from enum import Enum


class MessageType(Enum):
    """Standard message types for bank communications"""
    TRANSACTION_ALERT = "transaction_alert"
    SECURITY_ALERT = "security_alert"
    ACCOUNT_UPDATE = "account_update"
    MARKETING = "marketing"
    GENERAL = "general"


@dataclass
class SignedMessage:
    """A signed message ready to send to customers"""
    message_id: str
    sender_domain: str
    recipient: str
    message_type: str
    message_text: str
    sent_at: str
    signature: str
    public_key: str

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization"""
        return {
            "message_id": self.message_id,
            "sender_domain": self.sender_domain,
            "recipient": self.recipient,
            "message_type": self.message_type,
            "message_text": self.message_text,
            "sent_at": self.sent_at,
            "signature": self.signature,
            "public_key": self.public_key,
        }

    def to_notification_payload(self) -> Dict[str, Any]:
        """
        Convert to push notification payload format

        This is what you send via your push notification service
        (Firebase, APNS, etc.)
        """
        return {
            "title": f"Message from {self.sender_domain}",
            "body": self.message_text,
            "data": self.to_dict()
        }
