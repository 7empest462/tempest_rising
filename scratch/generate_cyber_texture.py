import math
import random
from PIL import Image, ImageDraw, ImageFilter

def generate_cyber_suit_texture():
    width = 1024
    height = 1024
    image = Image.new("RGBA", (width, height), (22, 28, 38, 255))
    draw = ImageDraw.Draw(image)

    # 1. Base metallic paneling grid
    panel_size = 128
    for y in range(0, height, panel_size):
        for x in range(0, width, panel_size):
            # Alternating subtle dark titanium tones
            tone = random.randint(24, 38)
            blue_tint = tone + random.randint(10, 20)
            color = (tone, tone + 4, blue_tint, 255)
            draw.rectangle([x, y, x + panel_size - 1, y + panel_size - 1], fill=color)

    # 2. Carbon-fiber micro weave grid pattern
    for y in range(0, height, 8):
        for x in range(0, width, 8):
            if (x // 8 + y // 8) % 2 == 0:
                draw.rectangle([x, y, x + 7, y + 7], fill=(14, 18, 26, 255))

    # 3. Hexagonal armor plating overlay
    hex_radius = 48
    def draw_hexagon(cx, cy, r, color, outline_color):
        points = []
        for i in range(6):
            angle = math.pi / 3 * i
            px = cx + r * math.cos(angle)
            py = cy + r * math.sin(angle)
            points.append((px, py))
        draw.polygon(points, fill=color, outline=outline_color)

    for row in range(-1, height // 60 + 2):
        for col in range(-1, width // 60 + 2):
            cx = col * 72 + (36 if row % 2 else 0)
            cy = row * 62
            c_val = random.randint(28, 42)
            hex_color = (c_val, c_val + 6, c_val + 16, 230)
            border_color = (15, 20, 30, 255)
            draw_hexagon(cx, cy, 32, hex_color, border_color)

    # 4. Sci-Fi panel seam borders & industrial rivet bolts
    seam_color = (10, 14, 20, 255)
    rivet_color = (140, 155, 175, 255)
    for x in range(0, width, panel_size):
        draw.line([(x, 0), (x, height)], fill=seam_color, width=3)
        for y in range(16, height, 32):
            draw.ellipse([x - 3, y - 3, x + 3, y + 3], fill=rivet_color)

    for y in range(0, height, panel_size):
        draw.line([(0, y), (width, y)], fill=seam_color, width=3)
        for x in range(16, width, 32):
            draw.ellipse([x - 3, x - 3, x + 3, x + 3], fill=rivet_color)

    # 5. Glowing Cyan Energy Circuit Lines & Plasma Power Traces
    cyan_glow = (0, 220, 240, 220)
    for i in range(16):
        x1 = random.randint(0, width)
        y1 = random.randint(0, height)
        dx = random.choice([-100, 0, 100])
        dy = random.choice([-100, 0, 100])
        x2 = max(0, min(width, x1 + dx))
        y2 = max(0, min(height, y1 + dy))
        draw.line([(x1, y1), (x2, y2)], fill=cyan_glow, width=4)
        draw.ellipse([x2 - 5, y2 - 5, x2 + 5, y2 + 5], fill=(80, 255, 240, 255))

    # 6. Burnished Copper/Gold Heat-Shield Accent Stripes
    gold_accent = (220, 140, 40, 200)
    for x in range(100, width, 300):
        draw.rectangle([x, 0, x + 24, height], fill=gold_accent)

    # 7. Add metallic micro-scratch noise overlay
    scratch_img = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    s_draw = ImageDraw.Draw(scratch_img)
    for _ in range(800):
        sx = random.randint(0, width)
        sy = random.randint(0, height)
        length = random.randint(10, 40)
        angle = random.uniform(0, math.pi)
        ex = sx + length * math.cos(angle)
        ey = sy + length * math.sin(angle)
        alpha = random.randint(40, 120)
        s_draw.line([(sx, sy), (ex, ey)], fill=(200, 220, 240, alpha), width=1)

    image = Image.alpha_composite(image, scratch_img)

    output_path = "assets/textures/cyber_suit_raw.png"
    image.save(output_path)
    print(f"Generated raw cyber suit texture at: {output_path}")

if __name__ == "__main__":
    generate_cyber_suit_texture()
