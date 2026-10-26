
import struct
import zlib

# 读取 RGB 数据
with open('rust_rgb_data.bin', 'rb') as f:
    rgb_data = f.read()

# 生成 PNG
png = b'\x89PNG\r\n\x1a\n'

# IHDR 块
ihdr_data = struct.pack('>IIBBBBB', 256, 240, 8, 2, 0, 0, 0)
ihdr_chunk = create_chunk(b'IHDR', ihdr_data)
png += ihdr_chunk

# IDAT 块（图像数据）
raw_data = b''
for y in range(240):
    raw_data += b'\x00'  # filter byte (None)
    for x in range(256):
        idx = (y * 256 + x) * 3
        raw_data += bytes([rgb_data[idx], rgb_data[idx+1], rgb_data[idx+2]])

compressed = zlib.compress(raw_data)
idat_chunk = create_chunk(b'IDAT', compressed)
png += idat_chunk

# IEND 块
iend_chunk = create_chunk(b'IEND', b'')
png += iend_chunk

# 保存 PNG
with open('rust_png_data.png', 'wb') as f:
    f.write(png)

def create_chunk(chunk_type, data):
    chunk = struct.pack('>I', len(data))
    chunk += chunk_type
    chunk += data
    crc = zlib.crc32(chunk_type + data) & 0xFFFFFFFF
    chunk += struct.pack('>I', crc)
    return chunk
