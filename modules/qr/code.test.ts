import jsQR from 'jsqr';
import { describe, expect, it } from 'vitest';
import { CAPACITY, encode, QUIET_ZONE, viewBoxOf, type Code } from './code';

/** The code drawn in black on white, 4 pixels a module, as a reader receives it. */
function pixelsOf(code: Code): { data: Uint8ClampedArray; side: number } {
  const scale = 4;
  const side = (code.size + QUIET_ZONE * 2) * scale;
  const data = new Uint8ClampedArray(side * side * 4).fill(255);
  for (let y = 0; y < side; y += 1) {
    for (let x = 0; x < side; x += 1) {
      const [mx, my] = [Math.floor(x / scale) - QUIET_ZONE, Math.floor(y / scale) - QUIET_ZONE];
      if (mx >= 0 && my >= 0 && mx < code.size && my < code.size && code.isDark(mx, my)) {
        data.fill(0, (y * side + x) * 4, (y * side + x) * 4 + 3);
      }
    }
  }
  return { data, side };
}

describe('code', () => {
  it.each(['https://github.com/arthreto/Arthex', 'WIFI:T:WPA;S:Maison;P:pâtes;;', 'Bonjour 👋'])(
    'is read back as it was written: %s',
    (content) => {
      const code = encode(content, 'M');
      if (!code) throw new Error('no code');
      const { data, side } = pixelsOf(code);
      expect(jsQR(data, side, side)?.data).toBe(content);
    },
  );

  it('counts bytes, not characters, and gives its version', () => {
    expect(encode('é', 'L')).toMatchObject({ bytes: 2, version: 1, size: 21 });
    expect(viewBoxOf(encode('é', 'L') as Code)).toBe('-4 -4 29 29');
  });

  it('refuses nothing and too much', () => {
    expect(encode('', 'M')).toBeNull();
    expect(encode('x'.repeat(CAPACITY.H + 1), 'H')).toBeNull();
    expect(encode('x'.repeat(CAPACITY.L), 'L')?.version).toBe(40);
  });
});
