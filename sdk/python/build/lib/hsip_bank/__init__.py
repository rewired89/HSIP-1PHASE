"""
HSIP Bank SDK - Backend message signing for banks
"""

from .signer import BankMessageSigner
from .keygen import generate_keypair, save_keypair, load_keypair
from .types import MessageType, SignedMessage

__version__ = "1.0.0"
__all__ = [
    "BankMessageSigner",
    "generate_keypair",
    "save_keypair",
    "load_keypair",
    "MessageType",
    "SignedMessage",
]
