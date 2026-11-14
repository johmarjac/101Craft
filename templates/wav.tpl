// 101Craft template: WAV audio (RIFF)
LittleEndian();

typedef struct {
    char id[4];
    uint size;
    char format[4];
} RIFFHEADER;

typedef struct {
    char id[4];
    uint size;
    if (id == "fmt ") {
        ushort audioFormat;
        ushort channels;
        uint   sampleRate;
        uint   byteRate;
        ushort blockAlign;
        ushort bitsPerSample;
        if (size > 16)
            uchar extra[size - 16];
    } else if (size > 0) {
        uchar data[Min(size, FileSize() - FTell())];
    }
    if (size % 2 == 1 && !FEof())
        uchar pad;
} RIFFCHUNK;

RIFFHEADER header;
while (!FEof()) {
    RIFFCHUNK chunk;
}
