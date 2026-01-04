"""
Message signing for HSIP bank communications
"""

import json
import uuid
from datetime import datetime
from typing import Optional
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from .types import MessageType, SignedMessage
from .keygen import load_keypair


class BankMessageSigner:
    """
    Signs messages from banks to customers using Ed25519

    Example:
        >>> signer = BankMessageSigner("keys/chase.com.private.pem", "chase.com")
        >>> signed = signer.sign_message(
        ...     "Transaction: $100.00 at Amazon",
        ...     MessageType.TRANSACTION_ALERT,
        ...     "customer@email.com"
        ... )
        >>> # Send signed message via push notification
        >>> send_push(signed.to_notification_payload())
    """

    def __init__(self, private_key_path: str, domain: str):
        """
        Initialize signer with bank's private key

        Args:
            private_key_path: Path to Ed25519 private key PEM file
            domain: Bank's domain (e.g., "chase.com")
        """
        self.domain = domain
        self.private_key, self.public_key = load_keypair(private_key_path)

        # Get public key as hex for distribution
        from cryptography.hazmat.primitives import serialization

        self.public_key_hex = self.public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        ).hex()

    def sign_message(
        self,
        message_text: str,
        message_type: MessageType,
        recipient: str,
        message_id: Optional[str] = None,
    ) -> SignedMessage:
        """
        Sign a message to send to a customer

        Args:
            message_text: The message content
            message_type: Type of message (transaction, security, etc.)
            recipient: Customer identifier (email, phone, user_id)
            message_id: Optional custom message ID (auto-generated if not provided)

        Returns:
            SignedMessage object ready to send

        Example:
            >>> signed = signer.sign_message(
            ...     "Your card was charged $50.00 at Starbucks",
            ...     MessageType.TRANSACTION_ALERT,
            ...     "user@example.com"
            ... )
        """
        # Generate message ID if not provided
        if message_id is None:
            message_id = f"msg_{uuid.uuid4().hex[:16]}"

        # Create message metadata
        sent_at = datetime.utcnow().isoformat() + "Z"

        # Build the data to sign (canonical format)
        sign_data = self._build_sign_data(
            message_id=message_id,
            sender_domain=self.domain,
            recipient=recipient,
            message_type=message_type.value,
            message_text=message_text,
            sent_at=sent_at,
        )

        # Sign the data
        signature_bytes = self.private_key.sign(sign_data)
        signature_hex = signature_bytes.hex()

        return SignedMessage(
            message_id=message_id,
            sender_domain=self.domain,
            recipient=recipient,
            message_type=message_type.value,
            message_text=message_text,
            sent_at=sent_at,
            signature=signature_hex,
            public_key=self.public_key_hex,
        )

    def _build_sign_data(
        self,
        message_id: str,
        sender_domain: str,
        recipient: str,
        message_type: str,
        message_text: str,
        sent_at: str,
    ) -> bytes:
        """
        Build canonical serialization for signing

        This must match the verification format in mobile apps
        """
        # Use a simple pipe-delimited format for now
        # In production, you might use protobuf or a more structured format
        parts = [
            "HSIP_V1",
            message_id,
            sender_domain,
            recipient,
            message_type,
            message_text,
            sent_at,
        ]
        canonical = "|".join(parts)
        return canonical.encode("utf-8")

    def get_public_key_hex(self) -> str:
        """
        Get the public key in hex format

        This is what you distribute to mobile apps for verification
        """
        return self.public_key_hex
