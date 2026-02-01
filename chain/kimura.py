#!/usr/bin/env python3
"""
Kimura - RPC Testing CLI
Tool for interacting with the Kimura blockchain via HTTP JSON-RPC

USAGE:
    python3 kimura.py message send --content "Hello" --sender node1
    python3 kimura.py block get --height 42
    python3 kimura.py height get
"""

import argparse
import json
import sys
import hashlib
import time
from pathlib import Path
from typing import Optional, Dict, Any

# Default RPC endpoint
DEFAULT_RPC_URL = "http://localhost:8545"

# Config file for RPC endpoint
CONFIG_FILE = Path.home() / ".kimura" / "config.json"

def load_config() -> Dict[str, Any]:
    """Load config from file or return defaults"""
    if CONFIG_FILE.exists():
        try:
            with open(CONFIG_FILE, 'r') as f:
                return json.load(f)
        except Exception:
            pass
    return {"rpc_url": DEFAULT_RPC_URL}

def save_config(config: Dict[str, Any]):
    """Save config to file"""
    CONFIG_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(CONFIG_FILE, 'w') as f:
        json.dump(config, f, indent=2)

def rpc_call(method: str, params: Dict[str, Any]) -> Dict[str, Any]:
    """Make JSON-RPC call to node"""
    import urllib.request
    
    config = load_config()
    rpc_url = config.get("rpc_url", DEFAULT_RPC_URL)
    
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    }
    
    try:
        req = urllib.request.Request(
            rpc_url,
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        
        with urllib.request.urlopen(req, timeout=10) as response:
            result = json.loads(response.read().decode())
            
            if "error" in result:
                print(f"RPC Error: {result['error']}")
                sys.exit(1)
            
            return result.get("result", {})
            
    except urllib.error.URLError as e:
        print(f"Connection error: {e}")
        print(f"Is the node running at {rpc_url}?")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

def generate_keypair():
    """Generate a secp256k1 keypair (simplified for MVP)"""
    # In production, use proper secp256k1 library
    # For MVP testing, we'll use a placeholder
    import secrets
    private_key = secrets.token_hex(32)
    public_key = hashlib.sha256(private_key.encode()).hexdigest()[:64]
    return private_key, public_key

def sign_message(content: str, nonce: int, private_key: str) -> str:
    """Sign message content (simplified for MVP)"""
    # In production, use secp256k1
    # For MVP, use simple HMAC-like signing
    message = f"{content}:{nonce}"
    signature = hashlib.blake2b(
        (message + private_key).encode(),
        key=private_key.encode()
    ).hexdigest()
    return signature

def main():
    parser = argparse.ArgumentParser(
        prog='kimura',
        description='RPC testing CLI for Kimura blockchain',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Commands:
  kimura message send --content "Hello" --sender node1
  kimura block get --height 42
  kimura height get
  kimura config set --rpc-url http://node.example.com:8545

Note: Requires node to be running with RPC enabled on port 8545
        '''
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Available commands')
    
    # Message subcommand
    msg_parser = subparsers.add_parser('message', help='Message operations')
    msg_subparsers = msg_parser.add_subparsers(dest='msg_action')
    
    msg_send = msg_subparsers.add_parser('send', help='Send a message to the blockchain')
    msg_send.add_argument('--content', '-c', required=True, help='Message content')
    msg_send.add_argument('--sender', '-s', required=True, help='Sender identifier')
    msg_send.add_argument('--nonce', '-n', type=int, help='Nonce (auto-generated if not provided)')
    
    # Block subcommand
    block_parser = subparsers.add_parser('block', help='Block operations')
    block_subparsers = block_parser.add_subparsers(dest='block_action')
    
    block_get = block_subparsers.add_parser('get', help='Get block by height')
    block_get.add_argument('--height', type=int, required=True, help='Block height')
    
    block_latest = block_subparsers.add_parser('latest', help='Get latest block')
    
    # Height subcommand
    height_parser = subparsers.add_parser('height', help='Chain height operations')
    height_subparsers = height_parser.add_subparsers(dest='height_action')
    
    height_get = height_subparsers.add_parser('get', help='Get current chain height')
    
    # Config subcommand
    config_parser = subparsers.add_parser('config', help='Configuration')
    config_subparsers = config_parser.add_subparsers(dest='config_action')
    
    config_set = config_subparsers.add_parser('set', help='Set configuration')
    config_set.add_argument('--rpc-url', help='RPC endpoint URL')
    
    config_get = config_subparsers.add_parser('get', help='Show current configuration')
    
    # Parse arguments
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        sys.exit(0)
    
    # Route commands
    if args.command == 'message':
        handle_message(args)
    elif args.command == 'block':
        handle_block(args)
    elif args.command == 'height':
        handle_height(args)
    elif args.command == 'config':
        handle_config(args)
    else:
        print(f"Unknown command: {args.command}")
        sys.exit(1)

def handle_message(args):
    """Handle message commands"""
    action = getattr(args, 'msg_action', None)
    
    if action == 'send':
        # For MVP, we'll use a simple approach
        # In production, load keys from secure storage
        content = args.content
        sender = args.sender
        nonce = args.nonce or int(time.time())  # Use timestamp as nonce if not provided
        
        # Generate temporary keypair (in production, load from file)
        private_key, public_key = generate_keypair()
        
        # Sign the message
        signature = sign_message(content, nonce, private_key)
        
        # Calculate message ID (blake3 of pubkey + nonce)
        message_id = hashlib.blake2b(
            f"{public_key}:{nonce}".encode()
        ).hexdigest()[:64]
        
        # Make RPC call
        params = {
            "sender": sender,
            "content": content,
            "signature": signature,
            "public_key": public_key,
            "nonce": nonce
        }
        
        print(f"Sending message: {content}")
        print(f"Sender: {sender}")
        print(f"Nonce: {nonce}")
        
        result = rpc_call("submit_message", params)
        
        print(f"\nMessage submitted successfully!")
        print(f"Message ID: {message_id}")
        print(f"Status: {result.get('status', 'unknown')}")
        
    else:
        print("Message action required (send)")
        sys.exit(1)

def handle_block(args):
    """Handle block commands"""
    action = getattr(args, 'block_action', None)
    
    if action == 'get':
        height = args.height
        
        print(f"Fetching block at height {height}...")
        
        result = rpc_call("get_block", {"height": height})
        
        if not result:
            print(f"Block {height} not found")
            sys.exit(1)
        
        header = result.get('header', {})
        message_ids = result.get('message_ids', [])
        
        print(f"\nBlock #{height}")
        print(f"  Timestamp: {header.get('timestamp', 'N/A')}")
        print(f"  Previous Hash: {header.get('prev_hash', 'N/A')[:32]}...")
        print(f"  Message Root: {header.get('message_root', 'N/A')[:32]}...")
        print(f"  Messages: {len(message_ids)}")
        
        if message_ids:
            print(f"\n  Message IDs:")
            for i, msg_id in enumerate(message_ids[:10]):  # Show first 10
                print(f"    {i+1}. {msg_id[:32]}...")
            if len(message_ids) > 10:
                print(f"    ... and {len(message_ids) - 10} more")
    
    elif action == 'latest':
        # Get current height first
        height_result = rpc_call("get_height", {})
        height = height_result.get('height', 0)
        
        if height == 0:
            print("No blocks yet (only genesis)")
            return
        
        # Get the latest block
        args.height = height
        handle_block(args)
    
    else:
        print("Block action required (get, latest)")
        sys.exit(1)

def handle_height(args):
    """Handle height commands"""
    action = getattr(args, 'height_action', None)
    
    if action == 'get':
        result = rpc_call("get_height", {})
        
        height = result.get('height', 0)
        hash_value = result.get('hash', 'N/A')
        
        print(f"Chain Height: {height}")
        print(f"Latest Hash: {hash_value[:64]}...")
    else:
        print("Height action required (get)")
        sys.exit(1)

def handle_config(args):
    """Handle config commands"""
    action = getattr(args, 'config_action', None)
    
    if action == 'set':
        config = load_config()
        
        if args.rpc_url:
            config['rpc_url'] = args.rpc_url
            print(f"Set RPC URL to: {args.rpc_url}")
        
        save_config(config)
        print("Configuration saved")
    
    elif action == 'get':
        config = load_config()
        
        print("Current Configuration:")
        print(f"  RPC URL: {config.get('rpc_url', DEFAULT_RPC_URL)}")
    
    else:
        print("Config action required (set, get)")
        sys.exit(1)

if __name__ == '__main__':
    main()