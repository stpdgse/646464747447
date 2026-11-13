/** Shrinks a picked image to a small square (cover-cropped) and returns it as a data URL under `maxChars`. Browser only. */
export async function pictureToDataUrl(file: File, maxChars = 80_000): Promise<string> {
  if (!/^image\/(png|jpeg|webp|gif)$/.test(file.type)) throw new Error("Pick a PNG, JPEG, WebP or GIF picture.");
  const bmp = await createImageBitmap(file);
  const side = Math.min(bmp.width, bmp.height);
  const sx = (bmp.width - side) / 2;
  const sy = (bmp.height - side) / 2;
  for (const size of [160, 128, 96, 64]) {
    const c = document.createElement("canvas");
    c.width = c.height = size;
    const x = c.getContext("2d");
    if (!x) break;
    x.imageSmoothingQuality = "high";
    x.drawImage(bmp, sx, sy, side, side, 0, 0, size, size);
    for (const [type, q] of [["image/png", undefined], ["image/webp", 0.85], ["image/jpeg", 0.8]] as const) {
      const url = c.toDataURL(type, q);
      if (url.startsWith(`data:${type}`) && url.length <= maxChars) return url;
    }
  }
  throw new Error("That picture is too detailed. Try a simpler one.");
}
