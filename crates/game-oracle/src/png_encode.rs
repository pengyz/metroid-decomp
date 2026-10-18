//! PNG 编码（8 位 RGB 非隔行——diff_visual 接受面）。
//!
//! 【结构互核注】①stored-deflate（无压缩块）=合法 zlib 流（python zlib 接受）——
//! 零依赖形 ②每行滤波字节 0（None）③块序：签名+IHDR+IDAT(zlib)+IEND——CRC32 逐块
//! +Adler32 数据尾。

const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(ctype: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 12);
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    let mut cdata = Vec::with_capacity(body.len() + 4);
    cdata.extend_from_slice(ctype);
    cdata.extend_from_slice(body);
    out.extend_from_slice(&cdata);
    out.extend_from_slice(&crc32(&cdata).to_be_bytes());
    out
}

/// 编码 8 位 RGB 非隔行 PNG（stored-deflate——每行滤波 0）。
pub fn encode_rgb(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    assert_eq!(pixels.len(), (width * height * 3) as usize);
    // 扫描线（滤波 0+RGB）——raw 数据
    let mut raw = Vec::with_capacity(height as usize * (1 + width as usize * 3));
    for y in 0..height as usize {
        raw.push(0);
        let s = y * width as usize * 3;
        raw.extend_from_slice(&pixels[s..s + width as usize * 3]);
    }
    // zlib 流（stored 块）
    let mut z = vec![0x78, 0x01];
    let mut off = 0;
    while off < raw.len() {
        let n = (raw.len() - off).min(65535);
        let last = (off + n) >= raw.len();
        z.push(if last { 1 } else { 0 });
        z.extend_from_slice(&(n as u16).to_le_bytes());
        z.extend_from_slice(&(!(n as u16)).to_le_bytes());
        z.extend_from_slice(&raw[off..off + n]);
        off += n;
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    // IHDR
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 位/RGB/压缩 0/滤波 0/非隔行
    // 组装
    let mut out = Vec::new();
    out.extend_from_slice(&SIG);
    out.extend_from_slice(&chunk(b"IHDR", &ihdr));
    out.extend_from_slice(&chunk(b"IDAT", &z));
    out.extend_from_slice(&chunk(b"IEND", &[]));
    out
}
