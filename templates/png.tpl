// 101Craft template: PNG image
BigEndian();

uchar signature[8] <format=hex>;

typedef struct {
    uint length;
    char type[4];
    if (type == "IHDR") {
        uint  width;
        uint  height;
        uchar bitDepth;
        uchar colorType;
        uchar compression;
        uchar filter;
        uchar interlace;
    } else if (type == "tEXt") {
        char text[length];
    } else if (length > 0) {
        uchar data[length];
    }
    uint crc <format=hex>;
} CHUNK;

while (!FEof()) {
    CHUNK chunk;
}
