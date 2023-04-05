#!/usr/bin/env python3
from getkey import getkey, keys

import asyncio
import websockets
import json
import random


from math import pi
from functools import wraps, partial


def async_wrap(func):
    @wraps(func)
    async def run(*args, loop=None, executor=None, **kwargs):
        if loop is None:
            loop = asyncio.get_event_loop()
        pfunc = partial(func, *args, **kwargs)
        return await loop.run_in_executor(executor, pfunc)

    return run


async_getkey = async_wrap(getkey)


async def consumer_handler(websocket, path):
    async for message in websocket:
        print(f"recv {message}")
        try:
            decoded = json.loads(message)
            bd = "BeatDuration"
            if bd in decoded:
                print("%.2f bpm" % ((1000 * 60) / int(decoded[bd])))
            else:
                print(message)
        except:
            print("that wasn't JSON")


i = 0


async def producer_handler(websocket, path):
    print("...")

    slerp = 0.05
    s = 0.9
    v = 0.1
    l = 8
    x = 0
    y = 0
    quit = False
    do_coro = True

    async def psv(data):
        await websocket.send(data)

    async def coro(which=0):
        gif = []
        while not quit and do_coro:
            for frame in gif:
                data = bytearray()
                try:
                    await websocket.send(data)
                except websockets.exceptions.ConnectionClosed:
                    print("closed¿")
                    return
                await asyncio.sleep(slerp)

    print("hi")

    while True:
        key = await async_getkey()
        if key == keys.ESC or key == keys.ENTER or key == keys.Q:
            print("bye")
            quit = True
            break
        elif key == keys.A:
            print("subscribing to admin")
            await websocket.send(json.dumps({"Subscribe": "Admin"}))
        elif key == keys.M:
            print("subscribing to midi")
            await websocket.send(json.dumps({"Subscribe": "Midi"}))
        elif key == keys.D:
            print("subscribing to dmx")
            await websocket.send(json.dumps({"Subscribe": {"Queue": "dmx"}}))
        elif key == keys.R:
            print("*resync*")
            await websocket.send('"Resync"')
        elif key == keys.L:
            print("*relatch*")
            await websocket.send('"Relatch"')
        # some pixelz for the matrixz
        elif key == keys.SPACE:
            await websocket.send('very long Soup')
        elif key == keys.X:
            await websocket.send('just x')
        elif key == keys.O:
            do_coro = True
            asyncio.create_task(coro(0))
        elif key == keys.K:
            do_coro = True
            asyncio.create_task(coro(1))
        elif key == keys.I:
            do_coro = False
        else:
            print("sending tarp")
            await websocket.send('"Tap"')


async def handler(websocket, path):
    consumer_task = asyncio.ensure_future(consumer_handler(websocket, path))
    producer_task = asyncio.ensure_future(producer_handler(websocket, path))
    done, pending = await asyncio.wait(
        [consumer_task, producer_task],
        return_when=asyncio.FIRST_COMPLETED,
    )
    for task in pending:
        task.cancel()


async def hello():
    import logging

    logger = logging.getLogger("websockets")
    # logger.setLevel(logging.DEBUG)
    # logger.addHandler(logging.StreamHandler())

    import os

    uri = os.environ.get("WS_ENDPOINT") or "ws://localhost:3030/"
    print(f"connecting to {uri}")
    async with websockets.connect(uri) as websocket:
        await handler(websocket, "wat")


asyncio.get_event_loop().run_until_complete(hello())
