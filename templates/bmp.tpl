// 101Craft template: Windows Bitmap (BMP)
LittleEndian();

typedef struct {
    char   bfType[2] <comment="Signature 'BM'">;
    uint   bfSize <comment="File size">;
    ushort bfReserved1;
    ushort bfReserved2;
    uint   bfOffBits <format=hex, comment="Start of the pixel data">;
} BITMAPFILEHEADER;

typedef struct {
    uint   biSize;
    int    biWidth;
    int    biHeight;
    ushort biPlanes;
    ushort biBitCount;
    uint   biCompression;
    uint   biSizeImage;
    int    biXPelsPerMeter;
    int    biYPelsPerMeter;
    uint   biClrUsed;
    uint   biClrImportant;
} BITMAPINFOHEADER;

typedef struct {
    uchar blue;
    uchar green;
    uchar red;
    uchar reserved;
} RGBQUAD;

BITMAPFILEHEADER bmfh;
BITMAPINFOHEADER bmih;

local int ncolors = bmih.biClrUsed;
if (ncolors == 0 && bmih.biBitCount <= 8)
    ncolors = 1 << bmih.biBitCount;
if (ncolors > 0)
    RGBQUAD aColors[ncolors];

FSeek(bmfh.bfOffBits);
local int rowsize = ((bmih.biWidth * bmih.biBitCount + 31) / 32) * 4;
local int rows = bmih.biHeight;
if (rows < 0) rows = -rows;

typedef struct {
    uchar pixels[rowsize];
} BITMAPLINE;

if (rows > 0 && rowsize > 0)
    BITMAPLINE lines[rows];
