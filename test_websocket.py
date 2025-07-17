#!/usr/bin/env python3
import asyncio
import websockets
import sys

async def test_websocket():
    uri = "ws://localhost:3000/demos/todo/"
    try:
        async with websockets.connect(uri) as websocket:
            print("WebSocket connected successfully!")
            
            # Wait for a moment to ensure connection is stable
            await asyncio.sleep(1)
            
            # Send a test message (though the server doesn't process it)
            await websocket.send("Hello WebSocket")
            
            # Wait a bit more
            await asyncio.sleep(1)
            
            print("WebSocket connection stable - no crashes!")
            
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(test_websocket())