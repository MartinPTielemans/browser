"""Draws the Windows icons in Assets/ from the Mac icon's two shapes (branding/ilo.icon): the green
dot and the ink stem on its light plate, rounded as Windows 11's app icons are. Run it again after
the icon changes; the PNGs and ilo.ico it writes are committed, as ilo.icns is.

    python3 -I icons.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

# branding/ilo.icon/icon.json and Assets/*.svg, on their 1024 canvas.
PLATE = (243, 240, 232, 255)
DOT = (14, 159, 85, 255)
STEM = (22, 20, 15, 255)
SIZE = 1024
RADIUS = 0.2 * SIZE


def icon() -> Image.Image:
    image = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((0, 0, SIZE - 1, SIZE - 1), RADIUS, fill=PLATE)
    draw.ellipse((512 - 92.2, 276.5 - 92.2, 512 + 92.2, 276.5 + 92.2), fill=DOT)
    draw.rounded_rectangle((445.4, 450.6, 445.4 + 133.1, 450.6 + 358.4), 66.6, fill=STEM)
    return image


def main() -> None:
    assets = Path(__file__).resolve().parent / "Assets"
    assets.mkdir(exist_ok=True)
    whole = icon()
    # The names AppxManifest.xml gives them, at the scale-100 sizes, which Windows scales from.
    for name, size in [("Square44x44Logo", 44), ("Square150x150Logo", 150), ("StoreLogo", 50)]:
        whole.resize((size, size), Image.LANCZOS).save(assets / f"{name}.png", optimize=True)
    sizes = [16, 20, 24, 32, 40, 48, 64, 256]
    whole.resize((256, 256), Image.LANCZOS).save(assets / "ilo.ico", sizes=[(s, s) for s in sizes])


main()
