// Prepara la mascota: quita el fondo blanco (relleno desde los bordes, sin tocar la cara crema
// porque la protege el contorno oscuro), recorta y exporta:
//   public/mascot.png        -> UI del launcher
//   src-tauri/app-icon.png   -> fuente para `npm run tauri icon`
// Uso: node scripts/prepare-mascot.mjs [assets-src/mascot-original.webp]
import sharp from "sharp";

const input = process.argv[2] ?? "assets-src/mascot-original.webp";
const OUTLINE = [58, 26, 16]; // #3A1A10

const { data, info } = await sharp(input).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
const { width: w, height: h } = info;
const px = (x, y) => (y * w + x) * 4;
const light = (i) => Math.min(data[i], data[i + 1], data[i + 2]);

// Flood fill de los píxeles claros conectados con el borde
const seen = new Uint8Array(w * h);
const stack = [];
for (let x = 0; x < w; x++) stack.push(x, 0, x, h - 1);
for (let y = 0; y < h; y++) stack.push(0, y, w - 1, y);
while (stack.length) {
  const y = stack.pop(), x = stack.pop();
  if (x < 0 || y < 0 || x >= w || y >= h || seen[y * w + x]) continue;
  const i = px(x, y);
  if (light(i) < 150) continue; // llegamos al contorno
  seen[y * w + x] = 1;
  const l = light(i);
  if (l > 242) data[i + 3] = 0;
  else {
    // Borde antialias: convierte la mezcla blanco/contorno en contorno semitransparente
    data[i + 3] = Math.round(Math.min(1, (255 - l) / (255 - OUTLINE[0])) * 255);
    [data[i], data[i + 1], data[i + 2]] = OUTLINE;
  }
  stack.push(x + 1, y, x - 1, y, x, y + 1, x, y - 1);
}

const cut = sharp(data, { raw: { width: w, height: h, channels: 4 } }).trim();
const trimmed = await cut.png().toBuffer();

await sharp(trimmed).resize({ height: 512, fit: "inside" }).png({ compressionLevel: 9 }).toFile("public/mascot.png");
await sharp(trimmed)
  .resize(860, 860, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
  .extend({ top: 82, bottom: 82, left: 82, right: 82, background: { r: 0, g: 0, b: 0, alpha: 0 } })
  .png()
  .toFile("src-tauri/app-icon.png");
console.log("✓ public/mascot.png y src-tauri/app-icon.png");
