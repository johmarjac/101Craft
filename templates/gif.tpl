// 101Craft template: GIF image (header and colour table)
LittleEndian();

typedef struct {
    char   signature[3];
    char   version[3];
    ushort width;
    ushort height;
    uchar  flags <format=hex>;
    uchar  backgroundColor;
    uchar  aspectRatio;
} GIFHEADER;

typedef struct {
    uchar red;
    uchar green;
    uchar blue;
} RGB;

GIFHEADER header;
if (header.flags & 0x80)
    RGB globalColors[1 << ((header.flags & 7) + 1)];

local int rest = FileSize() - FTell();
if (rest > 0)
    uchar blocks[rest];
