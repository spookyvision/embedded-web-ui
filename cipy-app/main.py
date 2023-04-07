import usb_cdc
import json
import board
import neopixel
from rainbowio import colorwheel
from time import sleep

pixels = neopixel.NeoPixel(board.NEOPIXEL, 1)


def to_components(i):
    rgb = colorwheel(i)
    return (rgb & 0xff, rgb >> 8 & 0xff, rgb >> 16 & 0xff)


data = None
with open('ui.json', 'rb') as fh:
    data = fh.read()
usb_cdc.data.write(data)
usb_cdc.data.write(b'\0')
bof = bytearray()
print("ready for action")
while True:
    sep = b'\0'
    recv = usb_cdc.data.read()
    bof.extend(recv)
    while sep in bof:
        split = bof.index(sep)

        try:
            s = bof[:split].decode()
            dec = json.loads(s)
            print(dec)
            if dec.get('Click') == 0:
                usb_cdc.data.write(data)
                usb_cdc.data.write(b'\0')
            elif dec.get('Click') == 1:
                pixels[0] = (10, 10, 10)
            elif dec.get('Click') == 2:
                pixels[0] = (0, 0, 0)
            else:
                slider = dec.get('Slider')
                if slider is not None:
                    [idx, val] = slider
                    pixels[0] = to_components(val)
        except:
            print("decode error")
        bof = bof[split+1:]
